use std::fmt::Write;

use super::dsl::bits_required;
use super::Trace;

const INPUT: &str = "INPUT";

#[derive(Debug, Clone)]
pub struct Component {
    /// The number of slots in the logical memory
    size: u64,
    /// Bitwidth of the stored data elements
    width: u64,
    /// Width of the indexing ports
    address_bit_width: u64,
    /// Number of parallel ports in the memory
    port_count: u64,
    /// the list of memory banks where the index corresponds to the input port
    banks: Vec<MemoryBank>,
}

impl Component {
    pub fn from_trace(banks: Vec<MemoryBank>, trace: &Trace) -> Self {
        Self {
            size: trace.size() as u64,
            width: trace.bitwidth() as u64,
            address_bit_width: trace.bits_required() as u64,
            port_count: trace.num_ports() as u64,
            banks,
        }
    }
    pub fn from_parse(size: u64, width: u64, banks: Vec<MemoryBank>) -> Self {
        Self {
            size,
            width,
            address_bit_width: bits_required(width as usize) as u64,
            port_count: banks.len() as u64,
            banks,
        }
    }

    fn emit_input_ports(&self) -> String {
        let mut w = String::new();
        let last_idx = self.banks.len() - 1;
        for (idx, _) in self.banks.iter().enumerate() {
            if idx != last_idx {
                write!(w, "bank_{}_addr:{}, ", idx, self.address_bit_width).unwrap()
            } else {
                write!(w, "bank_{}_addr:{}", idx, self.address_bit_width).unwrap()
            }
        }
        w
    }
    fn emit_output_ports(&self) -> String {
        let mut w = String::new();
        let last_idx = self.banks.len() - 1;
        for (idx, _) in self.banks.iter().enumerate() {
            if idx != last_idx {
                write!(w, "read_bank_{}_addr:{}, ", idx, self.width).unwrap()
            } else {
                write!(w, "read_bank_{}_addr:{}", idx, self.width).unwrap()
            }
        }
        w
    }
    fn emit_cells(&self) -> String {
        let mut w = String::new();
        for (idx, bank) in self.banks.iter().enumerate() {
            writeln!(
                w,
                "{}@external bank_{} = std_mem_d1({width}, {size}, {addr_width});",
                " ".repeat(8),
                idx,
                width = self.width,
                size = bank.size(),
                addr_width = self.address_bit_width
            )
            .unwrap();
        }
        w
    }
    fn emit_wires(&self) -> (String, String) {
        let mut w = String::new();
        let mut c = String::new();
        for (idx, bank) in self.banks.iter().enumerate() {
            let (ic, iw) = bank.emit_wires(idx, self.address_bit_width as usize);
            writeln!(w, "{}", iw).unwrap();
            writeln!(c, "{}", ic).unwrap();
        }
        (c, w)
    }

    pub fn emit_calyx_comp(&self) -> String {
        let mut w = String::new();
        let (translation_cells, wires) = self.emit_wires();
        // TODO: fix this nightmare
        writeln!(
            w,
            r#"
import "primitives/core.futil";
component mem_{size}_{port_count}({input_ports}) -> ({output_ports}) {{
    cells {{
{cells}
{translation_cells}
    }}
    wires {{
{wires}

    }}
    control {{}}
}}

"#,
            size = self.size,
            port_count = self.port_count,
            input_ports = self.emit_input_ports(),
            output_ports = self.emit_output_ports(),
            cells = self.emit_cells(),
            translation_cells = translation_cells,
            wires = wires,
        )
        .unwrap();
        w
    }

    pub fn vailidate(&self, trace: &Trace) -> bool {
        for line in trace.iter() {
            for (idx, request) in line.iter().enumerate() {
                if let Some(request) = request {
                    if !self.banks[idx].can_read(*request) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct MemoryBank {
    routing: TopLevelRoutingProgram,
    memory_layout: TopLevelMemoryLayout,
}

impl MemoryBank {
    pub fn new(routing: TopLevelRoutingProgram, memory_layout: TopLevelMemoryLayout) -> Self {
        Self {
            routing,
            memory_layout,
        }
    }
    pub fn size(&self) -> usize {
        self.memory_layout.size()
    }

    pub fn emit_wires(&self, bank_idx: usize, addr_width: usize) -> (String, String) {
        let mut c = String::new();
        let mut w = String::new();

        if let TopLevelRoutingProgram::Prog(SequenceRoutingProg::Prog(p)) = &self.routing {
            match p {
                TerminalRoutingProgram::RShift(rs) => {
                    writeln!(c, "rsh_{} = std_rsh({});", bank_idx, addr_width).unwrap();
                    writeln!(w, "rsh_{idx}.left = bank_{idx}_addr;", idx = bank_idx).unwrap();
                    writeln!(w, "rsh_{}.right = {}'d{};", bank_idx, addr_width, rs).unwrap();
                    writeln!(w, "bank_{idx}.addr0 = rsh_{idx}.out;", idx = bank_idx).unwrap();
                    writeln!(
                        w,
                        "read_bank_{idx}_addr = bank_{idx}.read_data;",
                        idx = bank_idx
                    )
                    .unwrap();
                }
                TerminalRoutingProgram::Add(a) => {
                    writeln!(c, "add_{} = std_add({});", bank_idx, addr_width).unwrap();
                    writeln!(w, "add_{idx}.left = bank_{idx}_addr;", idx = bank_idx).unwrap();
                    writeln!(w, "add_{}.right = {}'d{};", bank_idx, addr_width, a).unwrap();
                    writeln!(w, "bank_{idx}.addr0 = add_{idx}.out;", idx = bank_idx).unwrap();
                    writeln!(
                        w,
                        "read_bank_{idx}_addr = bank_{idx}.read_data;",
                        idx = bank_idx
                    )
                    .unwrap();
                }
                TerminalRoutingProgram::SubPortVal(v) => {
                    writeln!(c, "sub_{} = std_sub({});", bank_idx, addr_width).unwrap();
                    writeln!(w, "sub_{idx}.left = bank_{idx}_addr;", idx = bank_idx).unwrap();
                    writeln!(w, "sub_{}.right = {}'d{};", bank_idx, addr_width, v).unwrap();
                    writeln!(w, "bank_{idx}.addr0 = sub_{idx}.out;", idx = bank_idx).unwrap();
                    writeln!(
                        w,
                        "read_bank_{idx}_addr = bank_{idx}.read_data;",
                        idx = bank_idx
                    )
                    .unwrap();
                }
                TerminalRoutingProgram::SubValPort(v) => {
                    writeln!(c, "sub_{} = std_sub({});", bank_idx, addr_width).unwrap();
                    writeln!(w, "sub_{idx}.right = bank_{idx}_addr;", idx = bank_idx).unwrap();
                    writeln!(w, "sub_{}.left = {}'d{};", bank_idx, addr_width, v).unwrap();
                    writeln!(w, "bank_{idx}.addr0 = sub_{idx}.out;", idx = bank_idx).unwrap();
                    writeln!(
                        w,
                        "read_bank_{idx}_addr = bank_{idx}.read_data;",
                        idx = bank_idx
                    )
                    .unwrap();
                }
                TerminalRoutingProgram::Constant(_) => todo!(), // useless in elemental context
                TerminalRoutingProgram::Noop => {
                    writeln!(w, "bank_{idx}.addr0 = bank_{idx}_addr;", idx = bank_idx).unwrap();
                    writeln!(
                        w,
                        "read_bank_{idx}_addr = bank_{idx}.read_data;",
                        idx = bank_idx
                    )
                    .unwrap();
                }
            }
        } else {
            todo!("Cannot do more complex routing");
        }
        (c, w)
    }
}

#[derive(Debug, Clone)]
pub struct TopLevelMemoryLayout {
    mems: Vec<MemoryLayout>,
}

impl TopLevelMemoryLayout {
    pub fn new(mems: Vec<MemoryLayout>) -> Self {
        Self { mems }
    }
    pub fn size(&self) -> usize {
        self.mems.iter().map(|x| x.size()).sum()
    }
}

#[derive(Debug, Clone)]
pub enum MemoryLayout {
    Range {
        start: usize,
        finish: usize,
        stride: usize,
    },
}

#[macro_export]
macro_rules! memory {
    ($start:expr ; $end:expr ; $stride:expr) => {
        $crate::structures::MemoryLayout::new(
            $start,
            $end,
            ($stride).into(),
        )
    };

    ($start:expr ; $end:expr) => {
        memory!($start ; $end ; None)
    };
}

#[derive(Debug, Clone)]
pub enum TopLevelRoutingProgram {
    Switch(
        Vec<(Condition, SequenceRoutingProg)>,
        Box<SequenceRoutingProg>,
    ),
    Prog(SequenceRoutingProg),
}

#[derive(Debug, Clone)]
pub enum SequenceRoutingProg {
    Sequence(Vec<TerminalRoutingProgram>),
    Prog(TerminalRoutingProgram),
}

#[derive(Debug, Clone)]
pub enum TerminalRoutingProgram {
    RShift(usize),
    // these all contain the other value
    Add(u64),
    SubPortVal(u64),
    SubValPort(u64),
    Constant(u64),
    Noop,
}

#[derive(Debug, Clone)]
pub enum Condition {
    ComparisonPortVal(u64, ComparisonOperator),
    ComparisonValPort(u64, ComparisonOperator),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    LessThan,
    Equal,
    GreaterThan,
    NotEqual,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

// #[derive(Debug, Clone)]
// pub enum ShiftDirection {
//     Left,
//     Right,
// }

impl ComparisonOperator {
    pub fn eval(&self, left: &u64, right: &u64) -> bool {
        match self {
            ComparisonOperator::LessThan => left < right,
            ComparisonOperator::Equal => left == right,
            ComparisonOperator::GreaterThan => left > right,
            ComparisonOperator::NotEqual => left != right,
            ComparisonOperator::LessThanOrEqual => left <= right,
            ComparisonOperator::GreaterThanOrEqual => left >= right,
        }
    }
}

impl Condition {
    pub fn eval(&self, port_val: u64) -> bool {
        match self {
            Condition::ComparisonPortVal(val, op) => op.eval(&port_val, val),
            Condition::ComparisonValPort(val, op) => op.eval(val, &port_val),
            Condition::And(c1, c2) => c1.eval(port_val) && c2.eval(port_val),
            Condition::Or(c1, c2) => c1.eval(port_val) || c2.eval(port_val),
            Condition::Not(c1) => !c1.eval(port_val),
        }
    }
}

impl TerminalRoutingProgram {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            TerminalRoutingProgram::Add(v) => (port_val + v),
            TerminalRoutingProgram::SubPortVal(v) => (port_val - v),
            TerminalRoutingProgram::SubValPort(v) => (v - port_val),
            TerminalRoutingProgram::Constant(c) => *c,
            TerminalRoutingProgram::RShift(amount) => port_val >> amount,
            TerminalRoutingProgram::Noop => port_val,
        }
    }
}

impl SequenceRoutingProg {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            SequenceRoutingProg::Sequence(s) => s.iter().fold(port_val, |acc, x| x.eval(acc)),
            SequenceRoutingProg::Prog(p) => p.eval(port_val),
        }
    }
}

impl TopLevelRoutingProgram {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            TopLevelRoutingProgram::Switch(vec, default) => {
                for (cond, prog) in vec.iter() {
                    if cond.eval(port_val) {
                        return prog.eval(port_val);
                    }
                }
                default.eval(port_val)
            }
            TopLevelRoutingProgram::Prog(p) => p.eval(port_val),
        }
    }
}

impl MemoryBank {
    pub fn can_read(&self, index: usize) -> bool {
        let routed_index = self.routing.eval(index as u64);
        let result = self.memory_layout.get(&(routed_index as usize));
        result.map(|x| x == index).unwrap_or(false)
    }
}

impl From<TerminalRoutingProgram> for SequenceRoutingProg {
    fn from(p: TerminalRoutingProgram) -> Self {
        Self::Prog(p)
    }
}

impl From<SequenceRoutingProg> for TopLevelRoutingProgram {
    fn from(p: SequenceRoutingProg) -> Self {
        Self::Prog(p)
    }
}

impl From<TerminalRoutingProgram> for TopLevelRoutingProgram {
    fn from(p: TerminalRoutingProgram) -> Self {
        let p: SequenceRoutingProg = p.into();
        p.into()
    }
}

impl MemoryLayout {
    pub fn new(start: usize, finish: usize, stride: Option<usize>) -> Self {
        let stride = stride.unwrap_or(1);
        assert!(start < finish);
        assert!(stride != 0);

        Self::Range {
            start,
            finish,
            stride,
        }
    }

    #[inline]
    pub fn _contains(&self, target: &usize) -> bool {
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => target >= start && target < finish && ((target - start) % stride) == 0,
        }
    }

    pub fn _index_of(&self, target: &usize) -> Option<usize> {
        if self._contains(target) {
            let out = match self {
                MemoryLayout::Range { start, stride, .. } => (target - start) / stride,
            };
            return Some(out);
        }
        None
    }

    pub fn size(&self) -> usize {
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => ((finish - start) / stride) + 1,
        }
    }

    pub fn _gen_array(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.size());
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => {
                let mut current = *start;
                while current < *finish {
                    out.push(current);
                    current += stride;
                }

                debug_assert!(out
                    .iter()
                    .enumerate()
                    .all(|(i, x)| self._index_of(x).unwrap_or_else(|| panic!("{:?}", x)) == i));
                debug_assert!(out
                    .iter()
                    .enumerate()
                    .all(|(i, x)| self.get(&i).unwrap() == *x));

                out
            }
        }
    }

    pub fn _last_idx(&self) -> usize {
        self.size() - 1
    }

    pub fn get(&self, idx: &usize) -> Option<usize> {
        if *idx >= self.size() {
            return None;
        }
        match self {
            MemoryLayout::Range { start, stride, .. } => Some(start + (stride * idx)),
        }
    }
}

impl TopLevelMemoryLayout {
    pub fn _contains(&self, target: &usize) -> bool {
        self.mems.iter().any(|x| x._contains(target))
    }

    pub fn _index_of(&self, target: &usize) -> Option<usize> {
        let mut idx = 0;

        for mem in self.mems.iter() {
            if mem._contains(target) {
                idx += mem._index_of(target).unwrap();
                return Some(idx);
            } else {
                idx += mem.size();
            }
        }

        None
    }

    pub fn get(&self, idx: &usize) -> Option<usize> {
        let mut bottom_idx = 0_usize;
        for mem in self.mems.iter() {
            if idx - bottom_idx < mem.size() {
                return mem.get(&(idx - bottom_idx));
            } else {
                bottom_idx += mem.size();
            }
        }
        None
    }
}

impl From<MemoryLayout> for TopLevelMemoryLayout {
    fn from(mem: MemoryLayout) -> Self {
        Self { mems: vec![mem] }
    }
}

impl TerminalRoutingProgram {
    pub fn pretty_print(&self) -> String {
        match self {
            TerminalRoutingProgram::RShift(n) => format!("{} >> {}", INPUT, n),
            TerminalRoutingProgram::Add(n) => format!("{} + {}", INPUT, n),
            TerminalRoutingProgram::SubPortVal(n) => format!("{} - {}", INPUT, n),
            TerminalRoutingProgram::SubValPort(n) => format!("{} - {}", n, INPUT),
            TerminalRoutingProgram::Constant(n) => format!("{}", n),
            TerminalRoutingProgram::Noop => "NOOP".to_string(),
        }
    }
}

impl SequenceRoutingProg {
    pub fn pretty_print(&self, level: usize) -> String {
        match self {
            SequenceRoutingProg::Sequence(s) => {
                let mut string = String::new();
                write!(string, "[").unwrap();
                write!(string, "{}{}", " ".repeat(level * 4), s[0].pretty_print()).unwrap();
                for element in s.iter().skip(1) {
                    writeln!(
                        string,
                        ",{}{}",
                        " ".repeat(level * 4),
                        element.pretty_print()
                    )
                    .unwrap();
                }
                write!(string, "{}]", " ".repeat(level * 4)).unwrap();
                string
            }
            SequenceRoutingProg::Prog(p) => p.pretty_print(),
        }
    }
}

impl ComparisonOperator {
    pub fn pretty_print(&self) -> String {
        String::from(match self {
            ComparisonOperator::LessThan => "<",
            ComparisonOperator::Equal => "==",
            ComparisonOperator::GreaterThan => ">",
            ComparisonOperator::NotEqual => "!=",
            ComparisonOperator::LessThanOrEqual => "<=",
            ComparisonOperator::GreaterThanOrEqual => ">=",
        })
    }
}

impl Condition {
    pub fn pretty_print(&self) -> String {
        match self {
            Condition::ComparisonPortVal(val, op) => {
                format!("{} {} {}", INPUT, op.pretty_print(), val)
            }
            Condition::ComparisonValPort(val, op) => {
                format!("{} {} {}", val, op.pretty_print(), INPUT)
            }
            Condition::And(first, second) => {
                format!("({} && {})", first.pretty_print(), second.pretty_print())
            }
            Condition::Or(first, second) => {
                format!("({} || {})", first.pretty_print(), second.pretty_print())
            }
            Condition::Not(c) => format!("!({})", c.pretty_print()),
        }
    }
}

impl TopLevelRoutingProgram {
    pub fn pretty_print(&self, level: usize) -> String {
        match self {
            TopLevelRoutingProgram::Switch(cases, default) => {
                let mut string = String::new();
                writeln!(string, "{}switch {{", " ".repeat(level * 4)).unwrap();
                for (cond, prog) in cases {
                    writeln!(
                        string,
                        "{}\t{} -> {},",
                        " ".repeat(level * 4),
                        cond.pretty_print(),
                        prog.pretty_print(level + 1)
                    )
                    .unwrap();
                }
                writeln!(
                    string,
                    "{}\t -> {}",
                    " ".repeat(level * 4),
                    default.pretty_print(level + 1)
                )
                .unwrap();
                writeln!(string, "{}}}", " ".repeat(level * 4)).unwrap();
                string
            }
            TopLevelRoutingProgram::Prog(p) => p.pretty_print(level),
        }
    }
}

impl MemoryLayout {
    pub fn pretty_print(&self) -> String {
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => format!("[{}:{}:{}]", start, finish, stride),
        }
    }
}

impl TopLevelMemoryLayout {
    pub fn pretty_print(&self, level: usize) -> String {
        match self.mems.len() {
            0 => unreachable!(),
            1 => self.mems[0].pretty_print(),
            _ => {
                let mut string = String::new();
                writeln!(string, "{}[", " ".repeat(level * 4)).unwrap();
                write!(
                    string,
                    "{}\t{}",
                    " ".repeat(level * 4),
                    self.mems[0].pretty_print()
                )
                .unwrap();
                for x in self.mems.iter().skip(1) {
                    write!(string, ",\n{}\t{}", " ".repeat(level * 4), x.pretty_print()).unwrap();
                }
                writeln!(string, "{}\n]", " ".repeat(level * 4)).unwrap();
                string
            }
        }
    }
}

impl MemoryBank {
    pub fn pretty_print(&self, level: usize) -> String {
        let mut string = String::new();
        writeln!(string, "{}bank {{", " ".repeat(level * 4)).unwrap();
        writeln!(
            string,
            "{}\tlayout: {}",
            " ".repeat(level * 4),
            self.memory_layout.pretty_print(level + 1)
        )
        .unwrap();
        writeln!(
            string,
            "{}\ttranslation: {}",
            " ".repeat(level * 4),
            self.routing.pretty_print(level + 1)
        )
        .unwrap();
        writeln!(string, "{}}}", " ".repeat(level * 4)).unwrap();
        string
    }
}

impl Component {
    pub fn pretty_print(&self) -> String {
        let mut string = String::new();
        writeln!(string, "memory<{},{}> {{", self.width, self.size).unwrap();
        for bank in &self.banks {
            writeln!(string, "{}", bank.pretty_print(1)).unwrap();
        }
        writeln!(string, "}}").unwrap();
        string
    }
}
