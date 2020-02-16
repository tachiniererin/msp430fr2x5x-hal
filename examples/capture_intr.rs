#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

use core::cell::UnsafeCell;
use embedded_hal::digital::v2::ToggleableOutputPin;
use msp430::interrupt::{enable, free, Mutex};
use msp430_rt::entry;
use msp430fr2355::interrupt;
use msp430fr2x5x_hal::{
    capture::{CapCmpPeriph, CapTrigger, Capture, CaptureVector, TBxIV, TimerConfig, CCR1},
    clock::{DcoclkFreqSel, MclkDiv, SmclkDiv},
    gpio::*,
    prelude::*,
};
use void::ResultVoidExt;

#[cfg(debug_assertions)]
use panic_msp430 as _;

#[cfg(not(debug_assertions))]
use panic_never as _;

static CAPTURE: Mutex<UnsafeCell<Option<Capture<msp430fr2355::tb0::RegisterBlock, CCR1>>>> =
    Mutex::new(UnsafeCell::new(None));
static VECTOR: Mutex<UnsafeCell<Option<TBxIV<msp430fr2355::tb0::RegisterBlock>>>> =
    Mutex::new(UnsafeCell::new(None));
static RED_LED: Mutex<UnsafeCell<Option<Pin<Port1, Pin0, Output>>>> =
    Mutex::new(UnsafeCell::new(None));

// Connect push button input to P1.6. When button is pressed, red LED should toggle. No debouncing,
// so sometimes inputs are missed.
#[entry]
fn main() -> ! {
    if let Some(periph) = msp430fr2355::Peripherals::take() {
        let mut fram = periph.FRCTL.constrain();
        periph.WDT_A.constrain();

        let pmm = periph.PMM.freeze();
        let p1 = periph.P1.batch().config_pin0(|p| p.to_output()).split(&pmm);
        let red_led = p1.pin0;

        free(|cs| unsafe { *RED_LED.borrow(&cs).get() = Some(red_led) });

        let (_smclk, aclk) = periph
            .CS
            .constrain()
            .mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
            .smclk_on(SmclkDiv::_1)
            .aclk_vloclk()
            .freeze(&mut fram);

        let captures = periph
            .TB0
            .to_capture(TimerConfig::aclk(&aclk))
            .config_cap1_input_A(p1.pin6.to_alternate2())
            .config_cap1_trigger(CapTrigger::FallingEdge)
            .commit();
        let mut capture = captures.cap1;
        let vectors = captures.tbxiv;

        setup_capture(&mut capture);
        free(|cs| {
            unsafe { *CAPTURE.borrow(&cs).get() = Some(capture) }
            unsafe { *VECTOR.borrow(&cs).get() = Some(vectors) }
        });
        unsafe { enable() };
    }

    loop {}
}

fn setup_capture<T: CapCmpPeriph<C>, C>(capture: &mut Capture<T, C>) {
    capture.enable_interrupts();
}

#[interrupt]
fn TIMER0_B1() {
    free(|cs| {
        if let Some(vector) = unsafe { &mut *VECTOR.borrow(&cs).get() }.as_mut() {
            if let Some(capture) = unsafe { &mut *CAPTURE.borrow(&cs).get() }.as_mut() {
                match vector.interrupt_vector() {
                    CaptureVector::Capture1(cap) => {
                        if cap.interrupt_capture(capture).is_ok() {
                            if let Some(led) = unsafe { &mut *RED_LED.borrow(&cs).get() }.as_mut() {
                                led.toggle().void_unwrap();
                            }
                        }
                    }
                    _ => {}
                };
            }
        }
    });
}
