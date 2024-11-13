# Example

```rust
fn main() {
    let info_list = hid_kit::DeviceInfo::enumerate().unwrap();
    for info in info_list {
        println!(
            "{:?} {}:{}",
            info.product_string(),
            info.product_id(),
            info.vendor_id()
        );
        if info.product_string() == Some("USB Capture HDMI") {
            let device = info.open().unwrap();

            let mut buf = [0; 32];
            buf[0] = 17;
            device.get_input_report(&mut buf).unwrap();
            dbg!(buf);
        }
    }
}
```
