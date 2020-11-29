// (Full example with detailed comments in examples/01d_quick_example.rs)
//
// This example demonstrates clap's full 'custom derive' style of creating arguments which is the
// simplest method of use, but sacrifices some flexibility.
use async_std::path::PathBuf as AsyncPathBuf;
use async_std::prelude::*;
use clap::Clap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tide::{Body, Request, Response, Result, StatusCode};
use std::cell::Ref;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = "1.0", author = "ryan chai . <chai_xb@163.com>")]
struct Opts {
    /// Some input. Because this isn't an Option<T> it's required to be used
    input: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

pub struct ServeDir {
    prefix: String,
    dir: PathBuf,
}

impl ServeDir {
    /// Create a new instance of `ServeDir`.
    pub(crate) fn new(prefix: String, dir: PathBuf) -> Self {
        Self { prefix, dir }
    }
}

/// From tide source
#[async_trait::async_trait]
impl<State> tide::Endpoint<State> for ServeDir
where
    State: Clone + Send + Sync + 'static,
{
    async fn call(&self, req: Request<State>) -> Result {
        let path = req.url().path();
        let path = path.trim_start_matches(&self.prefix);
        let path = path.trim_start_matches('/');

        // 迭代产生的元素是路径中的segment
        // 这里处理当前或者父级路径的方法值得学习
        // for p in Path::new(path) {
        //     if p == OsStr::new(".") {
        //         continue;
        //     } else if p == OsStr::new("..") {
        //         file_path.pop();
        //     } else {
        //         file_path.push(&p);
        //     }
        // }

        let mut file_path = AsyncPathBuf::new();
        file_path.push(&self.dir);
        file_path.push(path);

        println!("{:?}", file_path);
        let file_path = file_path.canonicalize().await?;

        println!("2{:?}", file_path);

        if !file_path.exists().await {
            return Ok(Response::new(StatusCode::NotFound));
        }

        // 判断请求地址属于普通文件还是文件夹
        if file_path.is_file().await {
            let body = Body::from_file(&file_path).await?;
            let mut res = Response::new(StatusCode::Ok);
            res.set_body(body);
            Ok(res)
        } else {
            let mut html = String::from("");
            html.push_str("<ul>");
            let mut entries = (file_path.read_dir().await?);
            let root = file_path.as_path().parent().unwrap().to_str().unwrap_or_default();

            // OMG !
            // Some 和 Ok 可以嵌套
            // 使用Ref来Borrow，而不是Move
            while let Some(Ok(ref entry)) = entries.next().await {
                // temporary value is freed at the end of this statement
                let sub = entry.path();
                let sub = sub.to_str().unwrap_or_default();
                let filename = entry.file_name();
                let filename = filename.to_string_lossy();
                html.push_str(format!(
                    "<li><a href={}>{}</a></li>",
                    sub.trim_start_matches(&root)
                        .replace("\\\\", "/")
                        .replace("\\", "/")
                        .as_str()
                        .trim_start_matches("/"),
                    filename,
                ).as_str())
            }
            html.push_str("</ul>");
            let mut res = Response::new(StatusCode::Ok);
            res.set_content_type("text/html");
            res.set_body(Body::from(html));
            Ok(res)
        }
    }
}

/// override tide 默认的 serve_dir 方法
trait TideExt {
    fn serve_dir2(&mut self, dir: impl AsRef<Path> + std::fmt::Debug) -> std::io::Result<()>;
}

impl<'a, State: Clone + Send + Sync + 'static> TideExt for tide::Route<'a, State> {
    fn serve_dir2(&mut self, dir: impl AsRef<Path> + std::fmt::Debug) -> std::io::Result<()> {
        let dir = dir.as_ref().to_owned();
        let prefix = self.path().to_string();
        self.at("*").get(ServeDir::new(prefix, dir));
        Ok(())
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts: Opts = Opts::parse();
    let mut app = tide::new();

    app.at("/").serve_dir2(opts.input)?;

    app.listen("127.0.0.1:8000").await?;

    Ok(())
}

#[test]
fn test() {
    for entry in std::fs::read_dir(Path::new(".")).unwrap() {
        println!("{:#?}", entry);
    }
}
