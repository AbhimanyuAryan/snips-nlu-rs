error_chain! {
    foreign_links {
        Io(::std::io::Error);
        NdArray(::ndarray::ShapeError);
        Csv(::csv::Error);
        Zip(::zip::result::ZipError);
        Regex(::regex::Error);
        Rustling(::rustling_ontology::RustlingError);
        Crfsuite(::crfsuite::Error);
        Base64(::base64::DecodeError);
        PackedResources(::resources_packed::Error);
        SerdeJson(::serde_json::Error);
        Utf8Error(::std::str::Utf8Error) #[cfg(feature = "ffi")];
        NulError(::std::ffi::NulError) #[cfg(feature = "ffi")];
    }
}

impl<T> ::std::convert::From<::std::sync::PoisonError<T>> for Error {
    fn from(pe: ::std::sync::PoisonError<T>) -> Error {
        format!("Poisoning error: {:?}", pe).into()
    }
}
