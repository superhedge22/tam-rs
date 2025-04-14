extern crate csv;
extern crate ta;

// TODO: implement some integration tests

#[cfg(test)]
mod test {
    mod serde {
        use ta::indicators::SimpleMovingAverage;
        use ta::Next;

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
