use clap::Parser;
use libc::{mkfifo, umask, access};
use simple_logger::SimpleLogger;
use core::fmt::Display;
use std::fs::{metadata, read_to_string, set_permissions, write};
use std::ffi::CString;
use core::ops::Deref;
use std::io::Error;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, PermissionsExt,MetadataExt,chown};
use std::path::{PathBuf, Path};
use core::str::FromStr;
use core::fmt::{Formatter,Result as FMTResult};

/// Default path for the `charge_control_start_threshold` file
const DEFAULT_START_PATH: &str = "/sys/class/power_supply/BAT0/charge_control_start_threshold";
/// Default path for the `charge_control_end_threshold` file
const DEFAULT_END_PATH: &str = "/sys/class/power_supply/BAT0/charge_control_end_threshold";
/// Default path for the IPC pipe
const DEFAULT_PIPE_PATH: &str = "/tmp/battery_pipe";
/// Default permission bits for the IPC pipe
const DEFAULT_PIPE_PERMS: &str = "777";

#[derive(Debug,Clone)]
/// Wraps a u32 and allows parsing from octal strings
struct OctalPermissions{
    /// Wrapped u32
    inner:u32,
}

impl Display for OctalPermissions{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FMTResult {
        formatter.write_fmt(format_args!("{:03o}",self.inner))
    }
}
impl FromStr for OctalPermissions{
    type Err = String;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut sum=0u32;
        for char in string.trim().chars(){
            sum<<=3;
            sum+=char.to_digit(8).ok_or("Cannot parse non digit")?;
        };
        Ok(Self{inner:sum})
    }
}
impl Deref for OctalPermissions{
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn pathbuf_to_c_string(path:&Path)->CString{
    CString::new(path.as_os_str().as_bytes()).expect("Path should not contain null byte")
}

fn writable(path:&Path)->bool{
    let path_c = pathbuf_to_c_string(path);
    matches! (unsafe{access(path_c.as_ptr(), libc::W_OK)},0i32)
}

fn trim_if_some<'a>(v:Option<(&'a str,&'a str)>)->Option<(&'a str,&'a str)>{
    if let Some((v1,v2))=v{
        return Some((v1.trim(),v2.trim()))
    }
    None
}

#[derive(Parser, Debug)]
struct CliArgs {
    #[arg(short,long,default_value=DEFAULT_START_PATH)]
    start_path: PathBuf,
    #[arg(short,long,default_value=DEFAULT_END_PATH)]
    end_path: PathBuf,
    #[arg(short,long,default_value=DEFAULT_PIPE_PATH)]
    pipe_path: PathBuf,
    #[arg(long,value_parser = clap::value_parser!(OctalPermissions),default_value=DEFAULT_PIPE_PERMS)]
    pipe_permissions: OctalPermissions,
    #[arg(long)]
    pipe_uid: Option<u32>,
    #[arg(long)]
    pipe_gid: Option<u32>,

    #[arg(long)]
    default_start: Option<u8>,
    #[arg(long)]
    default_end: Option<u8>,
}

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .expect("Logger should be initializable in main function");

    log::debug!("Parsing CLI args");
    let args = CliArgs::parse();
    log::debug!("CLI Args: {:?}", args);

    log::debug!(
        "Checking if path exists: {}",
        args.pipe_path.to_string_lossy()
    );
    if let Ok(meta_data)= metadata(&args.pipe_path) {
        log::debug!("Path exists; Checking if path is a pipe");

        if !meta_data.file_type().is_fifo() {
            log::debug!(
                "File of wrong type ({:?}) at pipe path: {}",
                meta_data.file_type(),
                args.pipe_path.to_string_lossy()
            );
            return;
        };
        

        let wrong_ids = {
            args.pipe_uid.is_some_and(|uid|meta_data.uid()!=uid) ||
            args.pipe_gid.is_some_and(|gid|meta_data.gid()!=gid)
        };
        
        if wrong_ids{
            log::debug!("Pipe has wrong uid or gid");
            if let Err(err) = chown(args.pipe_path.clone(), args.pipe_uid, args.pipe_gid){
            
                log::error!("Could not change pipe owner: {err}");
                return;
            } else {
            
                let changes = match(args.pipe_uid,args.pipe_gid){
                    (Some(user_id),Some(group_id)) => format!("uid to {user_id} and gid to {group_id}"),
                    (Some(user_id),None) => format!("uid to {user_id}"),
                    (None,Some(group_id)) => format!("gid to {group_id}"),
                    (None,None) => Err("Logical fallacy detected!").expect("This should not run"),
                };
                log::info!("Changed pipe {changes}");
            }
        }


        log::debug!("Path is a pipe; Checking pipe permissions");
        let mut permissions = meta_data.permissions();
        if (permissions.mode() &0b111_111_111) == *args.pipe_permissions {
            log::debug!("Pipe permissions are correct: {:03o}", permissions.mode());
        } else {
            log::debug!("Pipe permissions are wrong: {:03o} != {:03o}", permissions.mode(), *args.pipe_permissions);
            permissions.set_mode(*args.pipe_permissions);
            if let Err(err) = set_permissions(&args.pipe_path, permissions.clone()) {
                log::error!("Error while setting pipe permissions: {err}");
                return;
            };
            log::debug!("Changed pipe permissions to: {:03o}", permissions.mode());
        }


    } else {
        log::debug!("Pipe does not exist");
        log::info!("Creating pipe at {}", args.pipe_path.to_string_lossy());
        log::debug!("Changing umask");
        let umask_p = unsafe { umask(0) };
        log::debug!("Changed umask from {:03o} to {:03o}", umask_p, 0);
        let pipe_path_c = pathbuf_to_c_string(&args.pipe_path);
        if unsafe { mkfifo(pipe_path_c.as_ptr(), *args.pipe_permissions) } == 0 {
            log::info!("Successfully created pipe");
        } else {
            let errno = Error::last_os_error()
                .raw_os_error()
                .expect("There should be an error");
            let err = 
                match errno {
                    libc::EACCES => "EACCESS: One of the directories in pathname did not allow search (execute) permission",
                    libc::EDQUOT => "EDQUOT: The user's quota of disk blocks or inodes on the file system has been exhausted",
                    libc::EEXIST => "EEXIST: pathname already exists. This includes the case where pathname is a symbolic link, dangling or not",
                    libc::ENAMETOOLONG => "ENAMETOOLONG: Either the total length of pathname is greater than PATH_MAX, or an individual filename component has a length greater than NAME_MAX. In the GNU system, there is no imposed limit on overall filename length, but some file systems may place limits on the length of a component",
                    libc::ENOENT => "ENOENT: A directory component in pathname does not exist or is a dangling symbolic link",
                    libc::ENOSPC => "ENOSPC: The directory or file system has no room for the new file",
                    libc::ENOTDIR => "ENOTDIR: A component used as a directory in pathname is not, in fact, a directory",
                    libc::EROFS => "EROFS: pathname refers to a read-only file system",
                    _=>"UNKNOWN ERROR"
                };
            log::error!("Error while creating pipe: ({errno}): {err}");
            return;
        }
    };
    
    if let Some(value) = args.default_start{
        log::debug!("Setting default charge start threshold of {value}");
        match write(args.start_path.clone(),value.to_string()){
            Ok(())=>log::info!("Successfully set default charge start threshold of {value}"),
            Err(err)=>{log::error!("Error while setting default charge start threshold: {err}")},
        }
    }else{
        log::debug!("Skipping default charge start threshold value");
    };
    if let Some(value) = args.default_end{
        log::debug!("Setting default charge end threshold of {value}");
        match write(args.end_path.clone(),value.to_string()){
            Ok(())=>log::info!("Successfully set default charge end threshold of {value}"),
            Err(err)=>{log::error!("Error while setting default charge end threshold: {err}")},
        }
    }else{
        log::debug!("Skipping default charge end threshold value");
    };


    log::debug!("Check if start path is writable");
    let start_writable=writable(&args.start_path);
    let end_writable=writable(&args.end_path);
    
    if !start_writable {
        log::warn!("Start path is not writable");
    }

    if !end_writable {
        log::warn!("end path is not writable");
    }
    if !start_writable && !end_writable{
        log::error!("Neither start nor end path is writable");
        return
    }
    

    log::info!("Starting IPC loop");
    loop {
        let cmd = match read_to_string(args.pipe_path.clone()){
            Ok(cmd) => cmd,
            Err(err) => {log::error!("Error while reading IPC command from pipe: {err}"); continue;}
        };

        if let Some((start, end)) = trim_if_some(cmd.split_once("..")){
            set_thresholds(&args, Some(start), Some(end))
        }
        else if let Some((mode, value)) = trim_if_some(cmd
            .split_once('='))
            {
            match mode {
                "start" => set_thresholds(&args, Some(value), None),
                "end" => set_thresholds(&args, None,Some(value)),
                _ => log::error!("Wrong mode provided"),
            }            
        }    
    }
}

fn set_thresholds(args: &CliArgs,start_o: Option<&str>,end_o: Option<&str>){
    if start_o.is_none() && end_o.is_none() {
        return log::error!("Neither start nor end threshold provided")
    }
    
    let mut atleast_one_parsable = false;
    if let Some(start_s) = start_o{
        if let Ok(start) = start_s.parse::<u8>(){
            match write(args.start_path.clone(), start.to_string()){
                Ok(()) => log::info!("Successfully set charge control start threshold to {start}%"),
                Err(e)=>log::error!("Error while setting charge control start threshold: {e}"),
            }
            atleast_one_parsable = true;
        }
    }
    if let Some(end_s) = end_o{
        if let Ok(end) = end_s.parse::<u8>(){
            match write(args.end_path.clone(), end.to_string()){
                Ok(()) => log::info!("Successfully set charge control end threshold to {end}%"),
                Err(e)=>log::error!("Error while setting charge control end threshold: {e}"),
            }
            atleast_one_parsable = true;
        }
    }

    if !atleast_one_parsable{
        return log::error!("Neither start nor end threshold parsable")
    }
}