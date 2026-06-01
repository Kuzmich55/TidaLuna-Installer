use crate::installer::step::{InstallStep, StepResult, SubLog};
use async_trait::async_trait;
use std::process::Command;

fn run_command(program: &str, args: &[&str]) -> Option<std::process::Output> {
    Command::new(program).args(args).output().ok()
}

fn kill_processes_matching_patterns(patterns: &[&str], sublog_callback: &(dyn Fn(SubLog) + Send + Sync)) -> bool {
    let current_pid = std::process::id();

    let Some(output) = run_command("ps", &["-A", "-o", "pid=,command="]) else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut killed_any = false;

    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        let Some((pid_str, command)) = trimmed.split_once(' ') else {
            continue;
        };

        let Ok(pid) = pid_str.trim().parse::<u32>() else {
            continue;
        };

        if pid == current_pid {
            continue;
        }

        let command_lower = command.trim_start().to_lowercase();
        if !patterns.iter().any(|pattern| command_lower.contains(&pattern.to_lowercase())) {
            continue;
        }

        let pid_string = pid.to_string();
        if let Some(kill_output) = run_command("kill", &["-TERM", &pid_string]) {
            if kill_output.status.success() {
                killed_any = true;
                sublog_callback(SubLog {
                    message: format!("Stopped process pid {} matching command: {}", pid, command.trim()),
                });
            }
        }
    }

    killed_any
}

pub struct KillTidalStep;

#[async_trait]
impl InstallStep for KillTidalStep {
    fn name(&self) -> &str {
        "Kill TIDAL"
    }

    async fn run(&self, sublog_callback: &(dyn Fn(SubLog) + Send + Sync)) -> StepResult {
        let os = std::env::consts::OS;

        sublog_callback(SubLog {
            message: format!("Detected OS: {}", os),
        });

        let mut executed = false;
        let mut killed_any = false;

        match os {
            "windows" => {
                sublog_callback(SubLog {
                    message: "Killing TIDAL process(es) (Windows)".into(),
                });

                for image in ["TIDAL.exe", "Tidal.exe", "tidal.exe", "Update.exe"] {
                    if let Some(output) = run_command("taskkill", &["/IM", image, "/T", "/F"]) {
                        executed = true;
                        if output.status.success() {
                            killed_any = true;
                            sublog_callback(SubLog {
                                message: format!("Stopped process image: {}", image),
                            });
                        }
                    }
                }
            }
            "macos" => {
                sublog_callback(SubLog {
                    message: "Killing TIDAL process(es) (macOS)".into(),
                });

                executed = true;
                killed_any = kill_processes_matching_patterns(&["TIDAL", "Tidal"], sublog_callback);
            }
            "linux" => {
                sublog_callback(SubLog {
                    message: "Killing TIDAL process(es) (Linux)".into(),
                });

                executed = true;
                killed_any = kill_processes_matching_patterns(&["tidal-hifi", "tidal"], sublog_callback);
            }
            _ => {
                return StepResult {
                    success: false,
                    message: "Unsupported operating system".into(),
                };
            }
        }

        if !executed {
            sublog_callback(SubLog {
                message: "Warning: no kill command could be executed on this system".into(),
            });
        } else if !killed_any {
            sublog_callback(SubLog {
                message: "No running TIDAL process found to kill".into(),
            });
        }

        sublog_callback(SubLog {
            message: "Kill TIDAL step completed".into(),
        });

        StepResult {
            success: true,
            message: "Kill TIDAL completed (non-fatal)".into(),
        }
    }
}