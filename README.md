# Motivation

Manually maintaining a .PYI for your PYO3 project sucks

# Setup

## Unpack

Place pyi-macros in the root of your PYO3 project.

## Modify the path

Replace `"INSERT_MODULE_NAME_HERE"` with the name of your PYO3 module.

## Dependencies

Make a dependency to the pyi_macros and add an optional feature:

```
[dependencies]
pyo3 = { version = "0.20.3", features = ["chrono"] }
pyi_macros = { path = "../pyi-macros",  optional = true }

[features]
pyi = ["pyi_macros"]
```

# Usage

```rust
#[cfg_attr(feature = "pyi", pyi_macros::pyi)]
#[pyclass]
pub enum MyEnum {
    /// Some doc string
    High,
    /// Some doc string
    Low
}


#[cfg_attr(feature = "pyi", pyi_macros::pyi)]
#[pyclass]
pub struct MyClass {
    /// Some doc string
    pub value: i64
}

#[cfg_attr(feature = "pyi", pyi_macros::pyi_impl)]
#[pyclass]
pub struct MyClass {
    /// Some doc string
    pub value: i64
}
```

# Create the PYI interface file:

`> cargo build --features pyi`

# Know limitations

1. Not all types are implemented yet
2. This does not work well with `--all-features` flags for other cargo commands
3.
