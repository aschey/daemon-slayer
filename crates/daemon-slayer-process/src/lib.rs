use bytesize::ByteSize;
use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use daemon_slayer_core::cli::Printer;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
use sysinfo::{
    Pid, PidExt, Process, ProcessExt, ProcessRefreshKind, RefreshKind, Signal, System, SystemExt,
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

#[derive(Clone, Debug, Serialize, strum::Display)]
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
    #[strum(serialize = "Uninterruptible Disk Sleep")]
    UninterruptibleDiskSleep,
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
            sysinfo::ProcessStatus::UninterruptibleDiskSleep => {
                ProcessStatus::UninterruptibleDiskSleep
            }
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
            RefreshKind::new()
                .with_processes(ProcessRefreshKind::everything())
                .with_memory(),
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
        proc.kill_with(Signal::Kill);
    }

    pub fn process_info(&mut self) -> Option<ProcessInfo> {
        self.system.refresh_memory();
        self.system.refresh_process(self.pid);
        let total_memory = self.system.total_memory();
        let all_processes = self.system.processes();
        self.system
            .process(self.pid)
            .map(|p| ProcessInfo::new(self.pid, p, all_processes, total_memory))
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
    pub child_processes: Vec<ProcessInfo>,
    start_time: DateTime<Utc>,
    cpu_usage: f32,
    total_memory: u64,
}

impl ProcessInfo {
    fn new(
        pid: Pid,
        process: &Process,
        all_processes: &HashMap<Pid, Process>,
        total_memory: u64,
    ) -> Self {
        let disk_usage = if cfg!(any(target_os = "windows", target_os = "freebsd")) {
            None
        } else {
            Some(process.disk_usage())
        };

        Self {
            name: process.name().to_owned(),
            args: process.cmd().to_owned(),
            exe: process.exe().to_owned(),
            pid: pid.as_u32(),
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
            total_memory,
            child_processes: all_processes
                .iter()
                .filter_map(|(cur_pid, cur_process)| {
                    if cur_process.parent() == Some(pid) {
                        Some(ProcessInfo::new(
                            *cur_pid,
                            cur_process,
                            all_processes,
                            total_memory,
                        ))
                    } else {
                        None
                    }
                })
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

    pub fn format_start_time(&self) -> String {
        let converted: DateTime<Local> = DateTime::from(self.start_time);
        converted.format("%Y-%m-%d %r").to_string()
    }

    pub fn format_runtime(&self) -> String {
        let run_time = self.run_time();
        let seconds = run_time.num_seconds() % 60;
        let minutes = (run_time.num_seconds() / 60) % 60;
        let hours = (run_time.num_seconds() / 60) / 60;
        format!("{hours:0>2}:{minutes:0>2}:{seconds:0>2}")
    }

    pub fn memory_percent(&self, precision: usize) -> String {
        format!(
            "{:.1$}%",
            (self.memory.as_u64() as f64 / self.total_memory as f64) * 100.0,
            precision
        )
    }

    pub fn pretty_print(&self) -> String {
        Printer::default()
            .with_line("Name", &self.name)
            .with_line("Args", self.args.join(" "))
            .with_line("Exe", self.exe.to_string_lossy().to_string())
            .with_line("Pid", self.pid.to_string())
            .with_line("CWD", self.cwd.to_string_lossy().to_string())
            .with_line("Root", self.root.to_string_lossy().to_string())
            .with_line("Memory", self.memory.to_string())
            .with_line("Memory Percent", self.memory_percent(2))
            .with_line("Virtual Memory", self.virtual_memory.to_string())
            .with_line(
                "Parent PID",
                self.parent_pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "N/A".to_owned()),
            )
            //.with_line("Disk Usage", self.disk_usage.map(|d| d.total_read_bytes))
            .with_line("Status", self.status.to_string())
            .with_line("Start Time", self.format_start_time())
            .with_line("Run Time", self.format_runtime())
            .with_line("Total CPU Usage", self.formatted_total_cpu_usage_percent(2))
            .with_line(
                "Avg CPU Usage per Core",
                self.formatted_avg_cpu_usage_percent_per_core(2),
            )
            //.with_line("Child Processes", text)
            .print()
    }
}
