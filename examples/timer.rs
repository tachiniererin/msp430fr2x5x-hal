#![no_std]

extern crate panic_msp430;

use embedded_hal::digital::v2::*;
use embedded_hal::prelude::*;
use msp430fr2x5x_hal::{
    clock::{DcoclkFreqSel, MclkDiv, SmclkDiv},
    prelude::*,
    timer::{TimerConfig, TimerDiv, TimerExDiv},
};
use nb::block;
use void::ResultVoidExt;

// 0.5 second on, 0.5 second off
fn main() {
    let periph = msp430fr2355::Peripherals::take().unwrap();

    let mut fram = periph.FRCTL.constrain();
    periph.WDT_A.constrain();

    let pmm = periph.PMM.freeze();
    let p1 = periph.P1.batch().config_pin0(|p| p.to_output()).split(&pmm);
    let mut p1_0 = p1.pin0;

    let (_smclk, aclk) = periph
        .CS
        .constrain()
        .mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
        .smclk_on(SmclkDiv::_1)
        .aclk_vloclk()
        .freeze(&mut fram);

    let parts = periph
        .TB0
        .to_timer(TimerConfig::aclk(&aclk).clk_div(TimerDiv::_2, TimerExDiv::_5));
    let mut timer = parts.timer;
    let mut subtimer = parts.subtimer2;

    timer.start(1000u16);
    subtimer.set_count(500u16);
    loop {
        block!(subtimer.wait()).void_unwrap();
        p1_0.set_high().void_unwrap();
        // first 0.5 s of timer countdown expires while subtimer expires, so this should only block
        // for 0.5 s
        block!(timer.wait()).void_unwrap();
        p1_0.set_low().void_unwrap();
    }
}
