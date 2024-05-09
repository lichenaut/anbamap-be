pub enum Country {
    UnitedStates,
    Canada,
    Mexico,
}

pub fn get_country(country: &str) -> Country {
    match country {
        "United States" => Country::UnitedStates,
        "Canada" => Country::Canada,
        "Mexico" => Country::Mexico,
        _ => (panic!("Invalid country"),)
    }
}