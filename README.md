[![crates.io](https://img.shields.io/crates/v/svisual.svg)](https://crates.io/crates/svisual)
[![crates.io](https://img.shields.io/crates/d/svisual.svg)](https://crates.io/crates/svisual)

# SVisual-rs

Base Rust structures and traits for [SVisual](https://github.com/Tyill/SVisual/) client.

For example of implementation see [svisual-stm32f1](https://github.com/burrbull/svisual-stm32f1/).

### Usage

Let's measure 2 variables each 100ms and send them after 15 values be measured.
```
    let (mut tx, _) = Serial::usart3(
        p.USART3,
        (tx, rx),
        &mut afio.mapr,
        Config::default().baudrate(9600.bps()),
        clocks,
        &mut rcc.apb1,
    )
    .split();

    let mut delay = Delay::new(cp.SYST, clocks);

    // Create new map with not more than 2 different signals and 15 values in package
    let mut sv_map = SVMap::<2, 15>::new();

    loop {
        for i in 0..30 {
            // Set value of first signal of integers
            sv_map.set("temp", 15 + i).ok();
            // Set value of second signal of floats
            sv_map.set("temp2", 14. - (i as f32) / 2.).ok();
            // Use next value cell
            sv_map.next(|s| {
                // if package is full, send package with module name
                block!(tx.send_package("TempMod", s)).ok();
            });
            // Wait
            delay.delay_ms(100u16);
        }
    }
```
