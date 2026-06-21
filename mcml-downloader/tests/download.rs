use std::{env, sync::Arc};

use mcml_base::file_item::{FileHash, FileItemObj, LaterRun};
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

#[tokio::test]
async fn test_download() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let run_dir = exe_dir.parent().unwrap().to_path_buf();

    mcml_log::start(&run_dir);
    mcml_config::init(&run_dir);
    mcml_net::init();

    let gui = GuiRun {};

    mcml_downloader::set_gui_handel(Box::new(gui));

    mcml_downloader::init(&run_dir).unwrap();
    mcml_downloader::start();

    let obj = FileItemObj {
        name: String::from("apache-tomcat-11.0.22.zip"),
        file: run_dir.join("apache-tomcat-11.0.22.zip"),
        url: String::from(
            "https://dlcdn.apache.org/tomcat/tomcat-11/v11.0.22/bin/apache-tomcat-11.0.22.zip",
        ),
        hash: FileHash::Sha512(String::from(
            "b08163a8d51455d3a7ba8e588b824d06718450439be9d461913afa4a978f249d82b07d6837ad07ada0991408c2d0f1ccfc7c85617fe874e387e1ad89b4f7c12d",
        )),
        later: LaterRun::None,
    };

    let res = mcml_downloader::run_download_task(vec![obj]).await;
    assert!(res);

    mcml_downloader::stop();
}
