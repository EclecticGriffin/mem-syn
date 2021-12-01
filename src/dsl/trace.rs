use serde::Deserialize;
use serde_json::{self, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct Trace {
    trace: Vec<Vec<Option<usize>>>,
}

impl Trace {
    pub fn parse_trace<S: AsRef<str>>(input: S) -> Result<Self> {
        let mut trace: Self = serde_json::from_str(input.as_ref())?;
        trace.normalize();
        Ok(trace)
    }

    /// removes trace lines which are all empty
    /// pads nones onto the end of lines which omit entries
    fn normalize(&mut self) {
        let trace = std::mem::take(&mut self.trace);
        self.trace = trace
            .into_iter()
            .filter(|x| x.iter().any(|x| x.is_some()))
            .collect();
        let ports_required = self.ports_required();

        for line in self.trace.iter_mut() {
            while line.len() < ports_required {
                line.push(None)
            }
        }
    }

    fn ports_required(&self) -> usize {
        self.trace.iter().map(|x| x.len()).max().unwrap_or_default()
    }

    pub fn num_ports(&self) -> usize {
        self.trace.get(0).map_or(0, |x| x.len())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<Option<usize>>> {
        self.trace.iter()
    }
}
