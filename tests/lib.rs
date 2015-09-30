

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn shell_cmd() {
        // It's easier to test this using the shell because
        //   of the LD_PRELOAD behavior.
        // ...but we could consider executing 'cargo test'
        //   twice, once for a baseline w/o the LD_PRELOAD
        //   and another with the LD_PRELOAD.
        let status = Command::new("./tests/test.sh")
                         .status()
                         .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

        assert!(status.success());
    }
}
