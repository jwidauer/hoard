pub fn copy(contents: String) {
    if contents.is_empty() {
        return;
    }

    if let Err(e) = cli_clipboard::set_contents(contents) {
        panic!("Error setting clipboard contents: {}", e);
    }
}
