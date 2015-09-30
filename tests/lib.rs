

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn tbd() {
        let status = Command::new("./tests/test.sh").status().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });

        assert!(status.success());
    }
}
