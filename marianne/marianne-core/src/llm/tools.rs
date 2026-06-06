use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub action: String,
    pub args: serde_json::Value,
}

pub fn is_path_allowed(requested_path: &str, allowed_root: &Option<String>) -> bool {
    let Some(root) = allowed_root else { return false; };
    if root == "C:\\" || root == "C:/" { return true; }
    
    let req = Path::new(requested_path).canonicalize().unwrap_or_else(|_| PathBuf::from(requested_path));
    let allowed = Path::new(root).canonicalize().unwrap_or_else(|_| PathBuf::from(root));
    
    req.starts_with(allowed)
}

pub async fn execute_tool(call: &ToolCall, allowed_root: &Option<String>) -> Result<String, String> {
    match call.action.as_str() {
        "read_file" => {
            let path = call.args.get("path").and_then(|v| v.as_str()).ok_or("Missing path parameter")?;
            if !is_path_allowed(path, allowed_root) {
                return Err("Access denied to this path".to_string());
            }
            tokio::fs::read_to_string(path).await.map_err(|e| e.to_string())
        }
        "write_file" => {
            let path = call.args.get("path").and_then(|v| v.as_str()).ok_or("Missing path parameter")?;
            let content = call.args.get("content").and_then(|v| v.as_str()).ok_or("Missing content parameter")?;
            if !is_path_allowed(path, allowed_root) {
                return Err("Access denied to this path".to_string());
            }
            if let Some(parent) = Path::new(path).parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            tokio::fs::write(path, content).await.map_err(|e| e.to_string())?;
            Ok(format!("Successfully wrote to {}", path))
        }
        "list_dir" => {
            let path = call.args.get("path").and_then(|v| v.as_str()).ok_or("Missing path parameter")?;
            if !is_path_allowed(path, allowed_root) {
                return Err("Access denied to this path".to_string());
            }
            let mut entries = tokio::fs::read_dir(path).await.map_err(|e| e.to_string())?;
            let mut res = String::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                res.push_str(&format!("{} {}\n", if is_dir { "[DIR]" } else { "[FILE]" }, name));
            }
            if res.is_empty() {
                Ok("Directory is empty".to_string())
            } else {
                Ok(res)
            }
        }
        "run_command" => {
            let cmd_str = call.args.get("command").and_then(|v| v.as_str()).ok_or("Missing command parameter")?;
            
            #[cfg(target_os = "windows")]
            let mut cmd = tokio::process::Command::new("cmd");
            #[cfg(target_os = "windows")]
            cmd.args(["/C", cmd_str]);

            #[cfg(not(target_os = "windows"))]
            let mut cmd = tokio::process::Command::new("sh");
            #[cfg(not(target_os = "windows"))]
            cmd.args(["-c", cmd_str]);

            if let Some(root) = allowed_root {
                if root != "C:\\" && root != "C:/" {
                    cmd.current_dir(root);
                }
            }

            match tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output()).await {
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if output.status.success() {
                        Ok(format!("STDOUT:\n{}", stdout))
                    } else {
                        Ok(format!("Exit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}", output.status, stdout, stderr))
                    }
                }
                Ok(Err(e)) => Err(format!("Failed to execute command: {}", e)),
                Err(_) => Err("Command timed out after 30 seconds".to_string()),
            }
        }
        "replace_file_content" => {
            let path = call.args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            let old_text = call.args.get("old_text").and_then(|v| v.as_str()).ok_or("Missing old_text")?;
            let new_text = call.args.get("new_text").and_then(|v| v.as_str()).ok_or("Missing new_text")?;
            
            if !is_path_allowed(path, allowed_root) {
                return Err("Access denied to this path".to_string());
            }
            
            let content = tokio::fs::read_to_string(path).await.map_err(|e| e.to_string())?;
            if !content.contains(old_text) {
                return Err("old_text not found in file".to_string());
            }
            let updated = content.replace(old_text, new_text);
            tokio::fs::write(path, updated).await.map_err(|e| e.to_string())?;
            Ok(format!("Successfully replaced text in {}", path))
        }
        "grep_search" => {
            let path = call.args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            let query = call.args.get("query").and_then(|v| v.as_str()).ok_or("Missing query")?;
            
            if !is_path_allowed(path, allowed_root) {
                return Err("Access denied to this path".to_string());
            }

            let mut results = String::new();
            let mut stack = vec![PathBuf::from(path)];
            let max_results = 50;
            let mut count = 0;
            
            let regex = regex_lite::Regex::new(query).map_err(|e| e.to_string())?;

            while let Some(current_path) = stack.pop() {
                if count >= max_results { break; }
                if let Ok(mut entries) = tokio::fs::read_dir(&current_path).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let ty = entry.file_type().await;
                        if let Ok(t) = ty {
                            if t.is_dir() {
                                let name = entry.file_name().to_string_lossy().to_string();
                                if name != "node_modules" && name != "target" && name != ".git" {
                                    stack.push(entry.path());
                                }
                            } else if t.is_file() {
                                if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                                    for (i, line) in content.lines().enumerate() {
                                        if regex.is_match(line) {
                                            results.push_str(&format!("{}:{}: {}\n", entry.path().display(), i + 1, line.trim()));
                                            count += 1;
                                            if count >= max_results { break; }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if results.is_empty() {
                Ok("No matches found.".to_string())
            } else {
                Ok(results)
            }
        }
        _ => Err(format!("Unknown tool action: {}", call.action))
    }
}
