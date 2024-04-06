#![no_std]
use esp_backtrace as _;
use critical_section::Mutex;
use core::cell::RefCell;
use esp32s3_hal::{
    clock::ClockControl, 
    peripherals::*, 
    prelude::*, 
    Delay, 
    IO, 
    gpio::{
        PushPull,
        PullDown,
        Input,
        Output,
        Gpio21,  // LED STAND R
        Gpio26,  // LED STAND Y
        Gpio48,  // LED STAND G
        Gpio4,   // SR04 ECHO
        Gpio5,   // SR04 TRIGGER
        Gpio35,  // ONBOARD LED
    },
    systimer::SystemTimer,
    i2c::I2C
};
//use esp_println::println; // ONLY FOR DEBUG COMMENT IT WHEN NOT USED



// BMP180 addresses
const BMP180_ADDR: u8 = 0x77;
const AC5_MSB_ADDR: u8 = 0xB2;
const AC6_MSB_ADDR: u8 = 0xB4;
const MC_MSB_ADDR: u8 = 0xBC;
const MD_MSB_ADDR: u8 = 0xBE;
const CTRL_MEAS_ADDR: u8 = 0xF4;
const MEAS_OUT_LSB_ADDR: u8 = 0xF7;
const MEAS_OUT_MSB_ADDR: u8 = 0xF6;


struct Coeffs {
    ac5: i16,
    ac6: i16,
    mc: i16,
    md: i16,
}



static DELAY: Mutex<RefCell<Option<Delay>>> = Mutex::new(RefCell::new(None));
static LED: Mutex<RefCell<Option<Gpio35<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));

static SR04_TRIGGER: Mutex<RefCell<Option<Gpio5<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));
static SR04_ECHO: Mutex<RefCell<Option<Gpio4<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));

static BMP180: Mutex<RefCell<Option<I2C<I2C0>>>> = Mutex::new(RefCell::new(None));
static BMP_COEFF: Mutex<RefCell<Option<Coeffs>>> = Mutex::new(RefCell::new(None));

static RED: Mutex<RefCell<Option<Gpio21<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));
static YELLOW: Mutex<RefCell<Option<Gpio26<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));
static GREEN: Mutex<RefCell<Option<Gpio48<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));


fn write_read_address(address: u8) -> i16{
    let mut resp:i16 = 0;
    let mut buffer = [0u8, 2];
    critical_section::with(|cs| {
        let mut bmp180_ = BMP180.borrow_ref_mut(cs);
        match bmp180_.as_mut() {
            Some(bmp180) => {
                bmp180.write_read(BMP180_ADDR, &[address], &mut buffer).unwrap();
                resp = ((buffer[0] as i16) << 8) | buffer[1] as i16;
            }
            _ => {}
        }
    });
    return resp;
}


fn get_coeff() -> Coeffs{
    let mut calib_coeffs = Coeffs {
        ac5: 0,
        ac6: 0,
        mc: 0,
        md: 0,
    };
    // Read AC5 AC6 MC and MD Calibration coefficients
    calib_coeffs.ac5 = write_read_address(AC5_MSB_ADDR);
    calib_coeffs.ac6 = write_read_address(AC6_MSB_ADDR);
    calib_coeffs.mc = write_read_address(MC_MSB_ADDR);
    calib_coeffs.md = write_read_address(MD_MSB_ADDR);
    return calib_coeffs;
}



fn setup() {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let delay = Delay::new(&clocks);

    let _ = SystemTimer::new(peripherals.SYSTIMER);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let led = io.pins.gpio35.into_push_pull_output();
    let trigger = io.pins.gpio5.into_push_pull_output();
    let echo = io.pins.gpio4.into_pull_down_input();

    let bmp180 = I2C::new(
        peripherals.I2C0,
        io.pins.gpio7, //sda
        io.pins.gpio6, //scl
        100u32.kHz(),
        &clocks
    );

    let red = io.pins.gpio21.into_push_pull_output();
    let yellow = io.pins.gpio26.into_push_pull_output();
    let green = io.pins.gpio48.into_push_pull_output();

    critical_section::with(|cs| {
        LED.borrow_ref_mut(cs).replace(led);
        DELAY.borrow_ref_mut(cs).replace(delay);

        SR04_ECHO.borrow_ref_mut(cs).replace(echo);
        SR04_TRIGGER.borrow_ref_mut(cs).replace(trigger);

        BMP180.borrow_ref_mut(cs).replace(bmp180);
        BMP_COEFF.borrow_ref_mut(cs).replace(get_coeff());

        RED.borrow_ref_mut(cs).replace(red);
        YELLOW.borrow_ref_mut(cs).replace(yellow);
        GREEN.borrow_ref_mut(cs).replace(green);
    })
}

fn blink(){
    critical_section::with(|cs| {
        let mut led_ = LED.borrow_ref_mut(cs);
        let mut delay_ = DELAY.borrow_ref_mut(cs);
        match(led_.as_mut(), delay_.as_mut()) {
            (Some(led), Some(delay)) => {
                let _ = led.toggle();
                delay.delay_ms(50u32);
                let _ = led.toggle();
            }
            (_, _) => {}
        }
        
    });
}


fn read_temp()-> i16{
    let mut buffer = [0u8, 2];
    let mut word: i16 = 0;
    critical_section::with(|cs| {
        let mut bmp180_ = BMP180.borrow_ref_mut(cs);
        let mut delay_ = DELAY.borrow_ref_mut(cs);
        match (bmp180_.as_mut(), delay_.as_mut()) {
            (Some(bmp180), Some(delay)) => {
                bmp180.write(BMP180_ADDR, &[CTRL_MEAS_ADDR, 0x2E]).unwrap();
                delay.delay_ms(5u32);
                bmp180.write(BMP180_ADDR, &[MEAS_OUT_MSB_ADDR]).unwrap();
                bmp180.read(BMP180_ADDR, &mut buffer).unwrap();
                word = (buffer[0] as i16) << 8;
                // Read Measurement LSB
                bmp180.write(BMP180_ADDR, &[MEAS_OUT_LSB_ADDR]).unwrap();
                bmp180.read(BMP180_ADDR, &mut buffer).unwrap();
                word |= buffer[0] as i16;
            }
            (_, _) => {}
        }
    });
    return word;
}


fn compute_temp(ut: i16, coeff: &mut Coeffs) -> f64 {
    let ac5 = coeff.ac5;
    let ac6 = coeff.ac6;
    let mc = coeff.mc;
    let md = coeff.md;
    let base: i32 = 2;
    let x1 = ((ut as i32 - ac6 as i32) * ac5 as i32) >> 15;
    let x2 = ((mc as i32) << 11) / (x1 + md as i32);
    return ((x1 + x2 + 8i32) >> 4) as f64 / 10.0; // / 10 because T is in 0.1 °C
}


fn bmp180() -> f64 {     
    let mut t: f64 = 0.0;
    critical_section::with(|cs| {
        let mut coeff_ = BMP_COEFF.borrow_ref_mut(cs);
        match coeff_.as_mut() {
            Some(coeff) => {
                t = compute_temp(read_temp(), coeff);
            }
            _ => {}
        }
    });
    return t;
}


// fn leds(scaler: u8) {
//     if scaler > 7 {
//         return ();
//     }

//     let is_red: bool = (scaler & 0b001) != 0;
//     let is_yellow: bool = (scaler & 0b010) != 0;
//     let is_green: bool = (scaler & 0b100) != 0;

//     unsafe {
//         if let Some(ref mut red_led) = RED {
//             if is_red {
//                 red_led.set_high().unwrap();
//             } else {
//                 red_led.set_low().unwrap();
//             }
//         }
        
//         if let Some(ref mut yellow_led) = YELLOW {
//             if is_yellow {
//                 yellow_led.set_high().unwrap();
//             } else {
//                 yellow_led.set_low().unwrap();
//             }
//         }

//         if let Some(ref mut green_led) = GREEN {
//             if is_green {
//                 green_led.set_high().unwrap();

//             } else {
//                 green_led.set_low().unwrap();
//             }
//         }
//     }
// }


fn sr04() -> f64 {
    let mut distance = -1.0;
    critical_section::with(|cs| {
        let mut echo_ = SR04_ECHO.borrow_ref_mut(cs);
        let mut trigger_ = SR04_TRIGGER.borrow_ref_mut(cs);
        let mut delay_ = DELAY.borrow_ref_mut(cs);
        match(echo_.as_mut(), trigger_.as_mut(), delay_.as_mut()){
            (Some(echo), Some(trigger), Some(delay)) => {
                let _ = trigger.set_high();
                delay.delay_ms(10u32);
                let _ = trigger.set_low();
                let start: f64 = SystemTimer::now() as f64;
                while echo.is_low().expect("Error reading echo high") {}
                let stop: f64 = SystemTimer::now() as f64;
                //while echo.is_high().expect("Error reading echo low") {}
                distance = 340.0 * (stop-start) / (1000000.0 * 2.0 * 10.0); // 10 tick per us so /10
            }
            (_, _, _) => {}
        }
    });
    return distance;
}


// fn sha256(data: f64)-> Array{
//     return Array {
//         _0: [0; 32],
//     };
// }


















// FFI

#[repr(C)]
struct Array {
    _0: [u8; 32],
}

#[no_mangle]
pub extern "C" fn ffi_setup() {
    setup();
}

#[no_mangle]
pub extern "C" fn ffi_bmp180() -> f64 {
    bmp180()
}

#[no_mangle]
pub extern "C" fn ffi_sr04() -> f64 {
    sr04()
}

#[no_mangle]
pub extern "C" fn ffi_blink() {
    blink();
}

// // TODO: HERE SHA
// #[no_mangle]
// pub extern "C" fn ffi_sha256(data: f64) -> Array{
//     return sha256(data);
// }

// #[no_mangle]
// pub extern "C" fn ffi_leds(scaler: u8) {
//     leds(scaler)
// }
