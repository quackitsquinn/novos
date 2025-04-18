/// An adapter for serial communication. This is used to abstract the serial port from the rest of the kernel.
pub trait SerialAdapter
where
    Self: Send + Sync,
{
    /// Send a byte over the serial port. Returns Some if sending the byte would block, None otherwise.
    fn send(&self, data: u8);
    /// Send a slice of bytes over the serial port. Returns Some if sending the slice would block, None otherwise.
    fn send_slice(&self, data: &[u8]);
    /// Read a byte from the serial port. Returns Some if reading the byte would block, None otherwise.
    fn read(&self) -> u8;
    /// Read a slice of bytes from the serial port. Returns Some if reading the slice would block, None otherwise.
    fn read_slice(&self, data: &mut [u8]) -> usize;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use super::*;

    pub struct TestSerialAdapter {
        input: Mutex<(usize, Vec<u8>)>,
        output: Mutex<Vec<u8>>,
    }

    impl TestSerialAdapter {
        pub fn new() -> Self {
            Self {
                input: Mutex::new((0, Vec::new())),
                output: Mutex::new(Vec::new()),
            }
        }

        pub fn set_input(&self, input: &[u8]) {
            *self.input.lock().unwrap() = (0, input.to_vec());
        }

        pub fn get_output(&self) -> Vec<u8> {
            self.output.lock().unwrap().clone()
        }

        pub fn clear_output(&self) {
            self.output.lock().unwrap().clear();
        }

        #[track_caller]
        pub fn assert_send(&self, count: usize) {
            let output = self.output.lock().unwrap().len();
            assert_eq!(
                output, count,
                "Expected {} bytes to be sent, but got {}",
                count, output,
            );
        }
        #[track_caller]
        pub fn assert_read(&self, count: usize) {
            let input = self.input.lock().unwrap().0;
            assert_eq!(
                input,
                count,
                "Expected {} bytes to be read, but got {}",
                input,
                self.input.lock().unwrap().0
            );
        }
    }

    impl SerialAdapter for TestSerialAdapter {
        fn send(&self, data: u8) {
            self.output.lock().unwrap().push(data);
        }

        fn send_slice(&self, data: &[u8]) {
            self.output.lock().unwrap().extend_from_slice(data);
        }

        fn read(&self) -> u8 {
            let (index, input) = &mut *self.input.lock().unwrap();
            if *index >= input.len() {
                return 0;
            }

            let byte = input[*index];
            *index += 1;
            byte
        }

        fn read_slice(&self, data: &mut [u8]) -> usize {
            let (index, input) = &mut *self.input.lock().unwrap();
            let len = data.len().min(input.len() - *index);
            data[..len].copy_from_slice(&input[*index..*index + len]);
            *index += len;
            len
        }
    }

    #[test]
    fn test_test_serial_adapter() {
        let adapter = TestSerialAdapter::new();
        adapter.set_input(&[1, 2, 3, 4, 5]);
        assert_eq!(adapter.read(), 1);
        assert_eq!(adapter.read(), 2);
        assert_eq!(adapter.read(), 3);
        assert_eq!(adapter.read(), 4);
        assert_eq!(adapter.read(), 5);
        assert_eq!(adapter.read(), 0);
    }

    #[test]
    fn test_test_serial_adapter_slice() {
        let adapter = TestSerialAdapter::new();
        adapter.set_input(&[1, 2, 3, 4, 5]);
        let mut data = [0; 3];
        assert_eq!(adapter.read_slice(&mut data), 3);
        assert_eq!(data, [1, 2, 3]);
        assert_eq!(adapter.read_slice(&mut data), 2);
        assert_eq!(data, [4, 5, 3]);
        assert_eq!(adapter.read_slice(&mut data), 0);
        assert_eq!(data, [4, 5, 3]);
    }

    #[test]
    fn test_test_serial_adapter_send() {
        let adapter = TestSerialAdapter::new();
        adapter.send(1);
        adapter.send(2);
        adapter.send(3);
        adapter.send(4);
        adapter.send(5);
        assert_eq!(adapter.get_output(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_test_serial_adapter_send_slice() {
        let adapter = TestSerialAdapter::new();
        adapter.send_slice(&[1, 2, 3]);
        adapter.send_slice(&[4, 5]);
        assert_eq!(adapter.get_output(), vec![1, 2, 3, 4, 5]);
    }
}
