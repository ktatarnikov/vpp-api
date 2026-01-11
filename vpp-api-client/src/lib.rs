use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "25_10")] {

        #[path = "../gen/25.10/src/mod.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod client;

        pub use client::*;

        #[cfg(test)]
        #[path = "../gen/25.10/tests/afunix_interface_test.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod tests_afunix;

        #[cfg(test)]
        #[path = "../gen/25.10/tests/blocking_interface_test.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod tests_blocking;

        #[cfg(test)]
        #[path = "../gen/25.10/tests/nonblocking_interface_test.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod tests_nonblocking;

    } else if #[cfg(feature = "25_06")] {

        #[path = "../gen/25.06/src/mod.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod client;

        pub use client::*;

        #[cfg(test)]
        #[path = "../gen/25.06/tests/afunix_interface_test.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod tests_afunix;

        #[cfg(test)]
        #[path = "../gen/25.06/tests/blocking_interface_test.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod tests_blocking;

        #[cfg(test)]
        #[path = "../gen/25.06/tests/nonblocking_interface_test.rs"]
        #[rustfmt::skip]
        #[allow(clippy::all)]
        pub mod tests_nonblocking;
    } else {
        compile_error!("You must enable exactly one version feature: e.g. `25_10` or `25_06`");
    }
}
