use std;
use std::collections::HashMap;
use std::os::raw::c_char;
use std::ffi::CStr;
use std::ascii::AsciiExt;
use hyper;

type Pointer = usize;

#[no_mangle]
extern {
    fn ice_glue_destroy_response(resp: Pointer);
    fn ice_glue_response_get_body(t: Pointer, len_out: *mut u32) -> *const u8;
    fn ice_glue_response_get_file(t: Pointer) -> *const c_char;

    fn ice_glue_old_destroy_header_iterator(itr_p: Pointer);

    fn ice_glue_response_get_cookie(t: Pointer, k: *const c_char) -> *const c_char;
    fn ice_glue_response_create_cookie_iterator(t: Pointer) -> Pointer;
    fn ice_glue_destroy_cookie_iterator(itr_p: Pointer);
    fn ice_glue_response_cookie_iterator_next(t: Pointer, itr_p: Pointer) -> *const c_char;

    fn ice_glue_response_get_header(t: Pointer, k: *const c_char) -> *const c_char;
    fn ice_glue_response_create_header_iterator(t: Pointer) -> Pointer;
    fn ice_glue_response_header_iterator_next(t: Pointer, itr_p: Pointer) -> *const c_char;

    fn ice_glue_response_get_status(t: Pointer) -> u16;

    pub fn ice_glue_async_endpoint_handler(id: i32, call_info: Pointer);
}

pub struct Response {
    handle: Pointer
}

impl Response {
    pub unsafe fn from_raw(handle: Pointer) -> Response {
        if handle == 0 {
            panic!("Got a null pointer");
        }
        Response {
            handle: handle
        }
    }

    pub fn get_body(&self) -> Vec<u8> {
        //return Vec::new();

        let mut body_len: u32 = 0;
        let raw_body = unsafe { ice_glue_response_get_body(self.handle, &mut body_len) };

        if raw_body.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw_body, body_len as usize).to_vec() }
        }
    }

    pub fn get_file(&self) -> Option<String> {
        let p = unsafe { CStr::from_ptr(ice_glue_response_get_file(self.handle)).to_str().unwrap() };
        match p.len() {
            0 => None,
            _ => Some(p.to_string())
        }
    }

    pub fn get_headers(&self) -> hyper::header::Headers {
        let itr = unsafe { ice_glue_response_create_header_iterator(self.handle) };
        let mut resp_headers = hyper::header::Headers::new();

        loop {
            unsafe {
                let key = ice_glue_response_header_iterator_next(self.handle, itr);
                if key.is_null() {
                    break;
                }
                let key = CStr::from_ptr(key);
                let value = ice_glue_response_get_header(self.handle, key.as_ptr());
                let key = transform_header_name(key.to_str().unwrap());
                let value = CStr::from_ptr(value).to_str().unwrap();
                resp_headers.set_raw(key, value);
            }
        }

        unsafe { ice_glue_old_destroy_header_iterator(itr); }
        resp_headers
    }

    pub fn get_cookies(&self) -> HashMap<String, String> {
        let itr = unsafe { ice_glue_response_create_cookie_iterator(self.handle) };
        let mut resp_cookies = HashMap::new();

        loop {
            unsafe {
                let key = ice_glue_response_cookie_iterator_next(self.handle, itr);
                if key.is_null() {
                    break;
                }
                let key = CStr::from_ptr(key);
                let value = ice_glue_response_get_cookie(self.handle, key.as_ptr());
                let key = key.to_str().unwrap();
                let value = CStr::from_ptr(value).to_str().unwrap();
                resp_cookies.insert(key.to_string(), value.to_string());
            }
        }

        unsafe { ice_glue_destroy_cookie_iterator(itr); }
        resp_cookies
    }

    pub fn get_status(&self) -> hyper::StatusCode {
        let raw_status = unsafe { ice_glue_response_get_status(self.handle) };
        match hyper::StatusCode::try_from(raw_status) {
            Ok(v) => v,
            Err(_) => hyper::StatusCode::InternalServerError
        }
    }
}

impl Drop for Response {
    fn drop(&mut self) {
        if self.handle == 0 {
            return;
        }

        unsafe { ice_glue_destroy_response(self.handle); }
        self.handle = 0;
    }
}

fn transform_header_name(v: &str) -> String {
    let mut ret = String::new();
    let mut upper_case = true;

    for ch in v.chars() {
        if upper_case {
            ret.push(ch.to_ascii_uppercase());
            upper_case = false;
        } else {
            ret.push(ch);
        }
        if ch == '-' {
            upper_case = true;
        }
    }

    ret
}
