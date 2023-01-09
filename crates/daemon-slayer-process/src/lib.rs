use bytesize::ByteSize;
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Serialize;
use std::path::PathBuf;
use sysinfo::{
    Pid, PidExt, Process, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt,
};
#[cfg(feature = "cli")]
pub mod cli;

#[derive(Clone, Debug, Serialize)]
#[readonly::make]
pub struct DiskUsage {
    pub total_written_bytes: ByteSize,
    pub written_bytes: ByteSize,
    pub total_read_bytes: ByteSize,
    pub read_bytes: ByteSize,
}

#[derive(Clone, Debug, Serialize, strum_macros::Display)]
pub enum ProcessStatus {
    Idle,
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Tracing,
    Dead,
    Wakekill,
    Waking,
    Parked,
    #[strum(serialize = "Lock Blocked")]
    LockBlocked,
    Unknown(u32),
}

impl ProcessStatus {
    fn from_sysinfo_status(source: sysinfo::ProcessStatus) -> Self {
        match source {
            sysinfo::ProcessStatus::Idle => ProcessStatus::Idle,
            sysinfo::ProcessStatus::Run => ProcessStatus::Running,
            sysinfo::ProcessStatus::Sleep => ProcessStatus::Sleeping,
            sysinfo::ProcessStatus::Stop => ProcessStatus::Stopped,
            sysinfo::ProcessStatus::Zombie => ProcessStatus::Zombie,
            sysinfo::ProcessStatus::Tracing => ProcessStatus::Tracing,
            sysinfo::ProcessStatus::Dead => ProcessStatus::Dead,
            sysinfo::ProcessStatus::Wakekill => ProcessStatus::Wakekill,
            sysinfo::ProcessStatus::Waking => ProcessStatus::Waking,
            sysinfo::ProcessStatus::Parked => ProcessStatus::Parked,
            sysinfo::ProcessStatus::LockBlocked => ProcessStatus::LockBlocked,
            sysinfo::ProcessStatus::Unknown(status) => ProcessStatus::Unknown(status),
        }
    }
}

pub struct ProcessManager {
    system: System,
    pid: Pid,
}

impl ProcessManager {
    pub fn new(pid: u32) -> Self {
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );
        Self {
            system,
            pid: Pid::from_u32(pid),
        }
    }

    pub fn kill(pid: u32) {
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );
        let proc = system.process(Pid::from_u32(pid)).unwrap();
        proc.kill();
    }

    pub fn process_info(&mut self) -> Option<ProcessInfo> {
        self.system.refresh_process(self.pid);
        self.system
            .process(self.pid)
            .map(|p| ProcessInfo::new(self.pid.as_u32(), p))
    }
}

#[derive(Clone, Debug, Serialize)]
#[readonly::make]
pub struct ProcessInfo {
    pub name: String,
    pub args: Vec<String>,
    pub exe: PathBuf,
    pub pid: u32,
    pub cwd: PathBuf,
    pub root: PathBuf,
    pub memory: ByteSize,
    pub virtual_memory: ByteSize,
    pub parent_pid: Option<u32>,
    pub disk_usage: Option<DiskUsage>,
    pub status: ProcessStatus,
    pub subprocesses: Vec<ProcessInfo>,
    start_time: DateTime<Utc>,
    cpu_usage: f32,
}

impl ProcessInfo {
    fn new(pid: u32, process: &Process) -> Self {
        let disk_usage = if cfg!(any(target_os = "windows", target_os = "freebsd")) {
            None
        } else {
            Some(process.disk_usage())
        };
        Self {
            name: process.name().to_owned(),
            args: process.cmd().to_owned(),
            exe: process.exe().to_owned(),
            pid,
            cwd: process.cwd().to_owned(),
            root: process.root().to_owned(),
            memory: ByteSize(process.memory()),
            virtual_memory: ByteSize(process.virtual_memory()),
            parent_pid: process.parent().map(|p| p.as_u32()),
            status: ProcessStatus::from_sysinfo_status(process.status()),
            start_time: Utc.timestamp_opt(process.start_time() as i64, 0).unwrap(),
            cpu_usage: process.cpu_usage(),
            disk_usage: disk_usage.map(|d| DiskUsage {
                total_written_bytes: ByteSize(d.total_written_bytes),
                total_read_bytes: ByteSize(d.total_read_bytes),
                written_bytes: ByteSize(d.written_bytes),
                read_bytes: ByteSize(d.read_bytes),
            }),
            subprocesses: process
                .tasks
                .iter()
                .map(|p| ProcessInfo::new(p.0.as_u32(), p.1))
                .collect(),
        }
    }

    pub fn run_time(&self) -> Duration {
        Utc::now().signed_duration_since(self.start_time)
    }

    pub fn status_message(&self) -> String {
        self.status.to_string()
    }

    pub fn formatted_total_cpu_usage_percent(&self, precision: usize) -> String {
        format!("{:.1$}%", self.total_cpu_usage_percent(), precision)
    }

    pub fn formatted_avg_cpu_usage_percent_per_core(&self, precision: usize) -> String {
        format!("{:.1$}%", self.avg_cpu_usage_percent_per_core(), precision)
    }

    pub fn total_cpu_usage_percent(&self) -> f32 {
        self.cpu_usage
    }

    pub fn avg_cpu_usage_percent_per_core(&self) -> f32 {
        self.cpu_usage / (num_cpus::get() as f32)
    }
}
