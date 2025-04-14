extern crate csv;
extern crate tam;

// TODO: implement some integration tests

#[cfg(test)]
mod test {
    mod serde {
        use tam::indicators::SimpleMovingAverage;
        use tam::Next;

        // Simple smoke test that serde works (not sure if this is really necessary)
        #[test]
        fn test_serde() {
            let mut macd = SimpleMovingAverage::new(20).unwrap();
            let bytes = bincode::serde::encode_to_vec(&macd, bincode::config::standard()).unwrap();
            let mut deserialized: SimpleMovingAverage = bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap().0;

            assert_eq!(deserialized.next(2.0), macd.next(2.0));
        }
    }
}
