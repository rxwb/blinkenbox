#![no_main]
#![no_std]

#[rtic::app(device = esp32c3, dispatchers=[FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod blinkenbox {
    use esp_backtrace as _;
    use esp_hal::gpio::{Event, Input, Level, Output, Pull};
    use esp_println::println;
    use rtic_monotonics::esp32c3::prelude::*;
    use rtic_sync::{channel::Receiver, channel::Sender, make_channel};

    #[derive(Debug)]
    struct InEvent {
        time: fugit::Instant<u64, 1, 16000000>,
        gpios: u32,
    }

    const CAPACITY: usize = 3;

    esp32c3_systimer_monotonic!(Mono);

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        inputs: [Input<'static>; 3],
        outputs: [Output<'static>; 6],
        gpio_rx: Receiver<'static, InEvent, CAPACITY>,
        gpio_tx: Sender<'static, InEvent, CAPACITY>,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let peripherals = esp_hal::init(esp_hal::Config::default());

        // Inputs
        let mut inputs = [
            Input::new(peripherals.GPIO8, Pull::Up),
            Input::new(peripherals.GPIO9, Pull::Up),
            Input::new(peripherals.GPIO10, Pull::Up),
        ];
        for input in inputs.iter_mut() {
            input.listen(Event::FallingEdge);
        }

        // Outputs
        let outputs = [
            Output::new(peripherals.GPIO11, Level::Low),
            Output::new(peripherals.GPIO1, Level::Low),
            Output::new(peripherals.GPIO6, Level::Low),
            Output::new(peripherals.GPIO7, Level::Low),
            Output::new(peripherals.GPIO20, Level::Low),
            Output::new(peripherals.GPIO21, Level::Low),
        ];

        // Communication
        let (gpio_tx, gpio_rx) = make_channel!(InEvent, CAPACITY);

        // Threads
        pin_setter::spawn().unwrap();

        (
            Shared {},
            Local {
                inputs,
                outputs,
                gpio_rx,
                gpio_tx,
            },
        )
    }

    #[task(priority = 1, local=[gpio_rx, outputs])]
    async fn pin_setter(cx: pin_setter::Context) {
        loop {
            match cx.local.gpio_rx.recv().await {
                Ok(msg) => {
                    println!("{}: {}", msg.time, msg.gpios);
                    if let Some(gpio) = cx.local.outputs.get_mut::<usize>(0) {
                        gpio.toggle();
                    }
                }
                Err(err) => println!("ERROR receiving message: {:?}", err),
            }
        }
    }

    #[task(priority = 2, binds=GPIO, local=[inputs, gpio_tx])]
    fn gpio_handler(cx: gpio_handler::Context) {
        let time = Mono::now();
        let mut gpios = 0;
        for (i, input) in cx.local.inputs.iter_mut().enumerate() {
            input.clear_interrupt();
            if input.is_high() {
                gpios |= 1 << i;
            }
        }

        let msg = InEvent { time, gpios };

        let res = cx.local.gpio_tx.try_send(msg);
        if let Err(err) = res {
            println!("ERROR sending from GPIO handler: {:?}", err);
        }
    }
}
