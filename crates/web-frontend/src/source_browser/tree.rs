use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::source_browser::data;

/// Main source browser component with sidebar tree and code viewer
#[allow(non_snake_case)]
pub fn SourceBrowser() -> impl IntoView {
    let current_path = RwSignal::new(String::new());
    let selected_file = RwSignal::new(None::<String>);

    let navigate = use_navigate();

    let back_to_home = move |_| {
        navigate("/", Default::default());
    };

    let navigate_up = move |_| {
        let current = current_path.get();
        if let Some(slash) = current.rfind('/') {
            current_path.set(current[..slash].to_string());
        } else {
            current_path.set(String::new());
        }
        selected_file.set(None);
    };

    let children = move || data::list_children(&current_path.get());

    let dirs = move || {
        children()
            .into_iter()
            .filter(|(_, is_dir)| *is_dir)
            .collect_view()
    };

    let files = move || {
        children()
            .into_iter()
            .filter(|(_, is_dir)| !*is_dir)
            .collect_view()
    };

    view! {
        <div class="source-browser">
            <div class="source-browser-header">
                <button class="btn-back" on:click=back_to_home>
                    "← Home"
                </button>
                <h2>"Source Code Browser"</h2>
                <span class="file-count">
                    {format!("{} files", data::get_files().len())}
                </span>
            </div>
            <div class="source-browser-body">
                <div class="file-tree-sidebar">
                    <div class="tree-path-bar">
                        <button class="btn-up" on:click=navigate_up>
                            "↑ Up"
                        </button>
                        <span class="current-path-text">
                            {move || {
                                let p = current_path.get();
                                if p.is_empty() { "./".to_string() } else { p }
                            }}
                        </span>
                    </div>
                    <div class="tree-list">
                        {dirs}
                        {files}
                    </div>
                </div>
                <div class="code-viewer-panel">
                    {move || {
                        let path_opt = selected_file.get();
                        let content_opt = path_opt.as_ref().and_then(|p| {
                            data::get_file_content(p).map(|s| s.to_string())
                        });
                        let html = match (path_opt, content_opt) {
                            (Some(path), Some(content)) => {
                                let fn_name = if let Some(slash) = path.rfind('/') {
                                    path[slash + 1..].to_string()
                                } else {
                                    path.clone()
                                };
                                let lines: Vec<(usize, String)> = content
                                    .lines()
                                    .enumerate()
                                    .map(|(i, line)| (i + 1, escape_html(line)))
                                    .collect();
                                let line_count = lines.len();
                                format!(
                                    "<div class='code-viewer'><div class='code-viewer-header'><span class='code-file-name'>{}</span><span class='code-line-count'>{} lines</span></div><div class='code-content'>{}</div></div>",
                                    escape_html(&fn_name),
                                    line_count,
                                    lines.into_iter().map(|(num, text)| {
                                        format!("<div class='code-line'><span class='line-number'>{}</span><span class='line-text'>{}</span></div>", num, text)
                                    }).collect::<Vec<_>>().join("")
                                )
                            }
                            (Some(_), None) => {
                                "<div class='no-file-selected'><p>File not found</p></div>".to_string()
                            }
                            (None, _) => {
                                "<div class='no-file-selected'><p>Select a file to view its contents</p></div>".to_string()
                            }
                        };
                        view! {
                            <div inner_html=html />
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

/// Renders a list of directory items
#[allow(non_snake_case, unused)]
fn dirs(current_path: RwSignal<String>, selected_file: RwSignal<Option<String>>) -> impl IntoView {
    let children = move || data::list_children(&current_path.get());

    view! {
        <For
            each=move || {
                children().into_iter().filter(|(_, is_dir)| *is_dir).enumerate().map(|(i, (name, _))| {
                    (i, name)
                })
            }
            key=|(i, _)| *i
            children=move |(i, name)| {
                let _ = i;
                let open_dir = {
                    let name = name.clone();
                    let current_path = current_path;
                    let selected_file = selected_file;
                    move |_| {
                        let current = current_path.get();
                        let new_path = if current.is_empty() {
                            name.clone()
                        } else {
                            format!("{}/{}", current, name)
                        };
                        current_path.set(new_path);
                        selected_file.set(None);
                    }
                };
                view! {
                    <div class="tree-item tree-dir" on:click=open_dir>
                        <span class="tree-icon">"📁"</span>
                        <span class="tree-name">{name}</span>
                    </div>
                }
            }
        />
    }
}

/// Renders a list of file items
#[allow(non_snake_case, unused)]
fn files(current_path: RwSignal<String>, selected_file: RwSignal<Option<String>>) -> impl IntoView {
    let children = move || data::list_children(&current_path.get());

    view! {
        <For
            each=move || {
                children().into_iter().filter(|(_, is_dir)| !*is_dir).enumerate().map(|(i, (name, _))| {
                    (i, name)
                })
            }
            key=|(i, _)| *i
            children=move |(i, name)| {
                let _ = i;
                let open_file = {
                    let name = name.clone();
                    let current_path = current_path;
                    let selected_file = selected_file;
                    move |_| {
                        let current = current_path.get();
                        let full_path = if current.is_empty() {
                            name.clone()
                        } else {
                            format!("{}/{}", current, name)
                        };
                        selected_file.set(Some(full_path));
                    }
                };
                view! {
                    <div class="tree-item tree-file" on:click=open_file>
                        <span class="tree-icon">"📄"</span>
                        <span class="tree-name">{name}</span>
                    </div>
                }
            }
        />
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
