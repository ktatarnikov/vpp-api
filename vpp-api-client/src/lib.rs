use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "25_10")] {

        #[path = "../gen/25.10/src/mod.rs"]
        pub mod client;

        pub use client::*;

        #[cfg(test)]
        #[path = "../gen/25.10/tests/interface_test.rs"]
        pub mod tests;

    } else if #[cfg(feature = "25_06")] {

        #[path = "../gen/25.06/src/mod.rs"]
        pub mod client;

        pub use client::*;

        #[cfg(test)]
        #[path = "../gen/25.06/tests/interface_test.rs"]
        pub mod tests;

    } else {
        compile_error!("You must enable exactly one version feature: e.g. `25_10` or `25_06`");
    }
}
