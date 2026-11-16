#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corrupt_profile_opens_a_spare_and_says_so() {
        let dir = std::env::temp_dir().join(format!("kaiwa-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("broken.kaiwa");
        let garbage = b"this is not a database, not even close".repeat(40);
        std::fs::write(&file, &garbage).unwrap();

        let mut spare = Backend::or_spare(Backend::open_at(file.clone(), false));

        let troubles = spare.take_troubles();
        assert!(matches!(troubles.as_slice(), [Trouble::Unopened(_)]));
        assert!(spare.take_troubles().is_empty(), "said once");
        // The spare works, and the broken file stays as it was.
        assert!(spare.profile().is_ok());
        assert_eq!(std::fs::read(&file).unwrap(), garbage);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
