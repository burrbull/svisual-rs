[![crates.io](https://img.shields.io/crates/v/svisual.svg)](https://crates.io/crates/svisual)
[![crates.io](https://img.shields.io/crates/d/svisual.svg)](https://crates.io/crates/svisual)

# SVisual-rs

Base Rust structures and traits for [SVisual](https://github.com/Tyill/SVisual/) client.

For example of implementation see [svisual-stm32f1](https://github.com/burrbull/svisual-stm32f1/).

### Usage

Let's measure 2 variables each 100ms and send them after 15 values be measured.
```
let serial = Serial::usart1(
        dp.USART1,
        (pa9, pa10),
        &mut afio.mapr,
        115_200.bps(),
        clocks,
        &mut rcc.apb2,
);

let mut sv = svisual::SV::<U2, U15>::new();

loop {
    for i in 0..30 {
        sv.add_float_value(b"temp", 15.+(i as f32)).ok();
        sv.add_float_value(b"temp2", 14.-(i as f32)/2.).ok();
        sv.next(|s| {
            tx.send_package_dma(b"TempMod", s);
        });
        delay.delay_ms(100u16);
    }
}
```