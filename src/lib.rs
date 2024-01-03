use std::{
    ffi::{c_char, c_int, c_void, CStr},
    ptr::null,
    time::{Duration, Instant},
};

#[link(name = "ipamir")]
extern "C" {
    fn ipamir_signature() -> *const c_char;
    fn ipamir_init() -> *const c_void;
    fn ipamir_release(solver: *const c_void);
    fn ipamir_add_hard(solver: *const c_void, lit_or_zero: i32);
    fn ipamir_add_soft_lit(solver: *const c_void, lit: i32, weight: u64);
    fn ipamir_assume(solver: *const c_void, lit: i32);
    fn ipamir_solve(solver: *const c_void) -> c_int;
    fn ipamir_val_obj(solver: *const c_void) -> u64;
    fn ipamir_val_lit(solver: *const c_void, lit: i32) -> i32;
    fn ipamir_set_terminate(
        solver: *const c_void,
        state: *const c_void,
        x: Option<extern "C" fn(state: *const c_void) -> c_int>,
    );

}

pub struct Solution<'a> {
    ipamir: &'a mut IPAMIR,
}

pub enum MaxSatResult<'a> {
    Timeout(Option<Solution<'a>>),
    Optimal(Solution<'a>),
    Unsat,
    Error,
}

pub struct IPAMIR {
    ptr: *const c_void,
}

impl IPAMIR {
    pub fn new() -> Self {
        let ptr = unsafe { ipamir_init() };
        assert!(ptr != null());
        IPAMIR { ptr }
    }

    pub fn signature(&self) -> &str {
        let c_buf: *const c_char = unsafe { ipamir_signature() };
        let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
        let str_slice: &str = c_str.to_str().unwrap();
        str_slice
    }

    pub fn add_soft_lit(&mut self, lit: i32, weight: u64) {
        unsafe { ipamir_add_soft_lit(self.ptr, lit, weight) };
    }

    pub fn add_clause(&mut self, lits: impl Iterator<Item = i32>) {
        for lit in lits {
            unsafe { ipamir_add_hard(self.ptr, lit) };
        }
        unsafe { ipamir_add_hard(self.ptr, 0) };
    }

    pub fn solve(
        &mut self,
        timeout: Option<Duration>,
        assumptions: impl Iterator<Item = i32>,
    ) -> MaxSatResult {
        for lit in assumptions {
            unsafe { ipamir_assume(self.ptr, lit) };
        }

        struct CallbackUserData {
            start_time: Instant,
            timeout: Duration,
        }
        let mut userdata: Option<CallbackUserData> = None;

        if let Some(timeout) = timeout {
            userdata = Some(CallbackUserData {
                start_time: Instant::now(),
                timeout,
            });

            extern "C" fn cb(state: *const c_void) -> c_int {
                let ptr = state as *const CallbackUserData;
                let user_data = unsafe { &*ptr };

                if user_data.start_time.elapsed() > user_data.timeout {
                    1
                } else {
                    0
                }
            }

            unsafe {
                ipamir_set_terminate(
                    self.ptr,
                    userdata.as_ref().unwrap() as *const CallbackUserData as *const c_void,
                    Some(cb),
                )
            }
        }

        let code = unsafe { ipamir_solve(self.ptr) };

        if userdata.is_some() {
            unsafe { ipamir_set_terminate(self.ptr, null(), None) };
        }

        if code == 0 {
            MaxSatResult::Timeout(None)
        } else if code == 10 {
            MaxSatResult::Timeout(Some(Solution { ipamir: self }))
        } else if code == 20 {
            MaxSatResult::Unsat
        } else if code == 30 {
            MaxSatResult::Optimal(Solution { ipamir: self })
        } else if code == 40 {
            MaxSatResult::Error
        } else {
            panic!("unrecogized return code from ipamir");
        }
    }
}

impl Drop for IPAMIR {
    fn drop(&mut self) {
        unsafe { ipamir_release(self.ptr) };
    }
}

impl<'a> Solution<'a> {
    pub fn get_objective_value(&self) -> u64 {
        unsafe { ipamir_val_obj(self.ipamir.ptr) }
    }

    pub fn get_literal_value(&self, lit: i32) -> i32 {
        unsafe { ipamir_val_lit(self.ipamir.ptr, lit) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let s = IPAMIR::new();
        println!("{}", s.signature());
    }
}
