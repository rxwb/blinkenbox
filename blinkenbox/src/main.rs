#![no_main]
#![no_std]

#[rtic::app(device = esp32c3, dispatchers=[FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod blinkenbox {
    use esp_backtrace as _;
    use esp_hal::gpio::{Event, Input, Level, Output, Pull};
    use esp_println::println;
    use rtic_sync::{channel::Receiver, channel::Sender, make_channel};

    const CAPACITY: usize = 3;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        buttons: [Input<'static>; 3],
        outputs: [Output<'static>; 6],
        gpio_rx: Receiver<'static, u8, CAPACITY>,
        gpio_tx: Sender<'static, u8, CAPACITY>,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let peripherals = esp_hal::init(esp_hal::Config::default());

        // Inputs
        let mut buttons = [
            Input::new(peripherals.GPIO8, Pull::Up),
            Input::new(peripherals.GPIO9, Pull::Up),
            Input::new(peripherals.GPIO10, Pull::Up),
        ];
        for button in buttons.iter_mut() {
            button.listen(Event::FallingEdge);
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
        let (gpio_tx, gpio_rx) = make_channel!(u8, CAPACITY);

        // Threads
        pin_setter::spawn().unwrap();

        (
            Shared {},
            Local {
                buttons,
                outputs,
                gpio_rx,
                gpio_tx,
            },
        )
    }

    #[task(priority = 1, local=[gpio_rx, outputs])]
    async fn pin_setter(cx: pin_setter::Context) {
        loop {
            if let Ok(msg) = cx.local.gpio_rx.recv().await {
                if let Some(gpio) = cx.local.outputs.get_mut::<usize>(msg as _) {
                    gpio.toggle();
                }
            } else {
                println!("ERROR receiving message");
            }
        }
    }

    #[task(priority = 2, binds=GPIO, local=[buttons, gpio_tx])]
    fn gpio_handler(cx: gpio_handler::Context) {
        for button in cx.local.buttons {
            button.clear_interrupt();
        }
        let res = cx.local.gpio_tx.try_send(0);
        if res.is_err() {
            println!("ERROR sending from GPIO handler: {:?}", res);
        }
    }
}
