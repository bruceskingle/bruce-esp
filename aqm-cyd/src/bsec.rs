// use core::ffi::{c_double, c_int};

// #[repr(C)]
// pub struct BsecOutput {
//     pub iaq: c_double,
//     pub co2_eq: c_double,
//     pub breath_voc: c_double,
// }

// unsafe extern "C" {
//     // Replace names with those in the headers you downloaded
//     pub fn bsec_init() -> c_int;
//     pub fn bsec_update_measurements(
//         temperature_c: c_double,
//         humidity_rh: c_double,
//         pressure_pa: c_double,
//         gas_resistance_ohm: c_double,
//         out: *mut BsecOutput,
//     ) -> c_int;
// }

// pub fn init() -> bool {
//     unsafe { bsec_init() == 0 }
// }




// V2

// use core::ffi::{c_int, c_uint, c_float};

// pub const BSEC_MAX_PHYSICAL_SENSOR: usize = 10;
// pub const BSEC_MAX_OUTPUT_SENSORS: usize = 16;

// #[repr(C)]
// pub struct BsecSensorConfig {
//     pub sensor_id: c_uint,
//     pub sample_rate: c_float,
// }

// #[repr(C)]
// pub struct BsecInput {
//     pub sensor_id: c_uint,
//     pub signal: c_float,
//     pub time_stamp: i64,
// }

// #[repr(C)]
// pub struct BsecOutput {
//     pub sensor_id: c_uint,
//     pub signal: c_float,
//     pub accuracy: u8,
//     pub time_stamp: i64,
// }

// unsafe extern "C" {

//     pub fn bsec_init() -> c_int;

//     pub fn bsec_update_subscription(
//         requested_virtual_sensors: *const BsecSensorConfig,
//         n_requested_virtual_sensors: c_uint,
//         required_sensor_settings: *mut BsecSensorConfig,
//         n_required_sensor_settings: *mut c_uint,
//     ) -> c_int;

//     pub fn bsec_do_steps(
//         inputs: *const BsecInput,
//         n_inputs: c_uint,
//         outputs: *mut BsecOutput,
//         n_outputs: *mut c_uint,
//     ) -> c_int;

//     pub fn bsec_set_state(
//         serialized_state: *const u8,
//         length: c_uint,
//     ) -> c_int;

//     pub fn bsec_get_state(
//         serialized_state: *mut u8,
//         n_serialized_state: c_uint,
//         work_buffer: *mut u8,
//         n_work_buffer: c_uint,
//     ) -> c_int;

//     pub fn bsec_set_configuration(
//         serialized_cfg: *const u8,
//         n_serialized_cfg: c_uint,
//         work_buffer: *mut u8,
//         n_work_buffer: c_uint,
//     ) -> c_int;
// }

// pub fn init() -> bool {
//     unsafe { bsec_init() == 0 }
// }


use core::ffi::c_void;
use core::ffi::{c_int, c_uint, c_float, c_double};

use esp_println::println;

pub const BSEC_SAMPLE_RATE_ULP: f32 = 0.003333; // ultra-low-power (300s)
pub const BSEC_SAMPLE_RATE_LP: f32  = 0.333333; // low-power (3s)

// --- FFI declarations from bsec_interface.h ---
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct bsec_version_t {
    pub major: u8,
    pub minor: u8,
    pub major_bugfix: u8,
    pub minor_bugfix: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct bsec_output_t {
    pub sensor_id: u8,
    pub signal: c_double,
    pub accuracy: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct bsec_input_t {
    pub sensor_id: u8,
    pub signal: c_double,
    pub time_stamp: i64,
}

unsafe extern "C" {
    pub fn bsec_init() -> c_int;
    pub fn bsec_get_version(version: *mut bsec_version_t) -> c_int;
    pub fn bsec_do_steps(
        inputs: *const bsec_input_t,
        num_inputs: u8,
        outputs: *mut bsec_output_t,
        num_outputs: *mut u8,
    ) -> c_int;
}

// ---------------- HIGH-LEVEL SAFE WRAPPER ----------------

#[derive(Debug)]
pub struct BsecOutput {
    pub iaq: f32,
    pub iaq_accuracy: u8,
    pub co2_equiv: f32,
    pub voc_equiv: f32,
}

pub struct Bsec {
    outputs: [bsec_output_t; 16],
}

impl Bsec {
    pub fn new() -> Self {
        unsafe { bsec_init(); }
        Self {
            outputs: [bsec_output_t { sensor_id: 0, signal: 0.0, accuracy: 0 }; 16],
        }
    }

    pub fn version(&self) -> bsec_version_t {
        let mut v = bsec_version_t { major: 0, minor: 0, major_bugfix: 0, minor_bugfix: 0 };
        unsafe { bsec_get_version(&mut v); }
        v
    }

    /// Feed raw BME680 sensor values into BSEC
    pub fn update(
        &mut self,
        timestamp_ns: i64,
        temperature_c: f32,
        humidity_pct: f32,
        pressure_pa: f32,
        gas_res_ohm: f32,
    ) -> Option<BsecOutput> {

        let inputs = [
            bsec_input_t { sensor_id: 1, signal: temperature_c as f64, time_stamp: timestamp_ns },
            bsec_input_t { sensor_id: 2, signal: humidity_pct as f64, time_stamp: timestamp_ns },
            bsec_input_t { sensor_id: 3, signal: pressure_pa as f64, time_stamp: timestamp_ns },
            bsec_input_t { sensor_id: 4, signal: gas_res_ohm as f64, time_stamp: timestamp_ns },
        ];

        let mut num_outputs: u8 = 0;

        unsafe {
            bsec_do_steps(
                inputs.as_ptr(),
                inputs.len() as u8,
                self.outputs.as_mut_ptr(),
                &mut num_outputs,
            );
        }

        let mut iaq = None;
        let mut iaq_acc = 0;
        let mut co2 = None;
        let mut voc = None;
println!("num_outputs={}", num_outputs);
        for o in &self.outputs[..num_outputs as usize] {

println!("o.sensor_id={}", o.sensor_id);
            match o.sensor_id {
                6 => { iaq = Some(o.signal as f32); iaq_acc = o.accuracy; }
                7 => { co2 = Some(o.signal as f32); }
                8 => { voc = Some(o.signal as f32); }
                _ => {}
            }
        }

        iaq.map(|iaq_value| BsecOutput {
            iaq: iaq_value,
            iaq_accuracy: iaq_acc,
            co2_equiv: co2.unwrap_or(0.0),
            voc_equiv: voc.unwrap_or(0.0),
        })
    }
}
