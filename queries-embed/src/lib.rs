extern crate libc;
extern crate queries_core;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

extern crate serde_json;

use std::ffi::{CStr, CString};
use std::sync::{Mutex};

use libc::c_char;
use libc::c_float;


mod errors {
    error_chain! {
          foreign_links {
                Core(::queries_core::Error);
                Serde(::serde_json::Error);
                Utf8Error(::std::str::Utf8Error);
                NulError(::std::ffi::NulError);
          }
    }

    impl<T> ::std::convert::From<::std::sync::PoisonError<T>> for Error {
        fn from(pe: ::std::sync::PoisonError<T>) -> Error {
            format!("Poisoning error: {:?}", pe).into()
        }
    }
}

use errors::*;

#[repr(C)]
pub struct Opaque(std::sync::Mutex<queries_core::IntentParser>);


#[repr(C)]
pub enum QUERIESRESULT {
    KO = 0,
    OK = 1,
}

macro_rules! wrap {
    ( $e:expr ) => { match $e {
        Ok(_) => {return QUERIESRESULT::OK; }
        Err(e) => {
            use std::io::Write;
            use error_chain::ChainedError;
            let stderr = &mut ::std::io::stderr();
            let errmsg = "Error writing to stderr";
            writeln!(stderr, "{}", e.display()).expect(errmsg);
            return QUERIESRESULT::KO;
        }
    }}
}

macro_rules! get_intent_parser {
    ($opaque:ident) => {{
        let client: &Opaque = unsafe { &*$opaque };
        client.0.lock()?
    }};
}

macro_rules! get_str {
    ($pointer:ident) => {{
        unsafe { CStr::from_ptr($pointer) }.to_str()?
    }};
}

#[no_mangle]
pub extern "C" fn intent_parser_create(root_dir: *const libc::c_char,
                                       client: *mut *mut Opaque) -> QUERIESRESULT {
    wrap!(create(root_dir, client));
}


#[no_mangle]
pub extern "C" fn intent_parser_run_intent_classification(client: *mut Opaque,
                                                          input: *const c_char,
                                                          probability_threshold: c_float,
                                                          result_json: *mut *mut c_char)
                                                          -> QUERIESRESULT {
    wrap!(run_intent_classification(client, input, probability_threshold, result_json))
}

#[no_mangle]
pub extern "C" fn intent_parser_run_tokens_classification(client: *mut Opaque,
                                                          input: *const c_char,
                                                          intent_name: *const c_char,
                                                          result_json: *mut *mut c_char)
                                                          -> QUERIESRESULT {
    wrap!(run_tokens_classification(client, input, intent_name, result_json))
}


#[no_mangle]
pub extern "C" fn intent_parser_destroy_string(string: *mut libc::c_char) -> QUERIESRESULT {
    unsafe {
        let _string: CString = CString::from_raw(string);
    }

    QUERIESRESULT::OK
}


#[no_mangle]
pub extern "C" fn intent_parser_destroy_client(client: *mut Opaque) -> QUERIESRESULT {
    unsafe {
        let _parser: Box<Opaque> = Box::from_raw(client);
    }

    QUERIESRESULT::OK
}

fn create(root_dir: *const libc::c_char, client: *mut *mut Opaque) -> Result<()> {
    let root_dir = get_str!(root_dir);
    let file_configuration = queries_core::FileConfiguration::new(root_dir);
    let intent_parser = queries_core::IntentParser::new(&file_configuration, None)?;

    unsafe { *client = Box::into_raw(Box::new(Opaque(Mutex::new(intent_parser)))) };

    Ok(())
}

fn run_intent_classification(client: *mut Opaque,
                             input: *const c_char,
                             probability_threshold: c_float,
                             result_json: *mut *mut c_char) -> Result<()> {
    let input = get_str!(input);
    let intent_parser = get_intent_parser!(client);

    let results = intent_parser.run_intent_classifiers(input, probability_threshold as f64, None);

    point_to_string(result_json, serde_json::to_string(&results)?)
}

fn run_tokens_classification(client: *mut Opaque,
                             input: *const c_char,
                             intent_name: *const c_char,
                             result_json: *mut *mut c_char) -> Result<()> {
    let input = get_str!(input);
    let intent_name = get_str!(intent_name);
    let intent_parser = get_intent_parser!(client);


    let result = intent_parser.run_tokens_classifier(input, intent_name)?;

    point_to_string(result_json, serde_json::to_string(&result)?)
}

fn point_to_string(pointer: *mut *mut libc::c_char, string: String) -> Result<()> {
    let cs = CString::new(string.as_bytes())?;
    unsafe { *pointer = cs.into_raw() }
    Ok(())
}

