//! Capped pipeline log retained so a late-opened log window can catch up.

const DEFAULT_CAP: usize = 4000;

#[derive(Debug, Default)]
pub struct LogBuffer {
    lines: Vec<String>,
    cap: usize,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            cap: DEFAULT_CAP,
        }
    }

    /// Appends a line and drops the oldest entries when over cap.
    pub fn push(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        let extra = self.lines.len().saturating_sub(self.cap.max(1));
        if extra > 0 {
            self.lines.drain(0..extra);
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_preserves_order() {
        let mut buf = LogBuffer::new();
        buf.push("a");
        buf.push("b");
        assert_eq!(buf.snapshot(), ["a", "b"]);
    }

    #[test]
    fn cap_drops_oldest() {
        let mut buf = LogBuffer {
            lines: Vec::new(),
            cap: 2,
        };
        buf.push("a");
        buf.push("b");
        buf.push("c");
        assert_eq!(buf.snapshot(), ["b", "c"]);
    }

    #[test]
    fn clear_empties() {
        let mut buf = LogBuffer::new();
        buf.push("a");
        buf.clear();
        assert!(buf.snapshot().is_empty());
    }
}
