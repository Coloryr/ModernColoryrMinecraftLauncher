use std::{env, path::Path, sync::Arc};

use mcml_downloader::{DownloadGui, DownloadTaskState, download_item::DownloadItem};

struct GuiRun {}

impl DownloadGui for GuiRun {
    fn update(&self, thread: u32, file: &Arc<DownloadItem>) {
        // let pro = file.progress() as u64;
        // if pro > 0 && pro % 10 == 0 {
        //     println!(
        //         "线程 {thread} 下载项目 {} {}/{} {}%",
        //         file.base.name,
        //         file.get_now_size(),
        //         file.get_all_size(),
        //         file.progress()
        //     );
        // }
    }

    fn update_task(&self, state: DownloadTaskState) {
        match state {
            DownloadTaskState::AddTask(task) => {
                println!("下载状态 新建下载任务 {}", task)
            }
            DownloadTaskState::RemoveTask(task) => {
                println!("下载状态 删除下载任务 {}", task)
            }
            DownloadTaskState::UpdateTask(task_state_obj) => {
                println!(
                    "下载状态 更新下载任务 {} 进度 {}",
                    task_state_obj.id, task_state_obj.progress
                )
            }
        }
    }
}

fn start(run_dir: &Path) {
    mcml_log::start(&run_dir);
    mcml_config::init(&run_dir);
    mcml_net::init();

    let gui = GuiRun {};

    mcml_downloader::set_gui_handel(Box::new(gui));

    mcml_downloader::init(&run_dir).unwrap();
    mcml_downloader::start();
}

#[tokio::test]
async fn start_game() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap().to_path_buf();

    start(&run_dir);
}
