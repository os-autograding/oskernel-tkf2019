
use core::cell::RefCell;

use alloc::{string::{String, ToString}, vec::Vec, rc::{Rc, Weak}};
use fatfs::{Read, Write};

use crate::{device::{DiskFile, Dir}, runtime_err::RuntimeError};

use super::{file::{FileType, File}, cache::get_cache_file, virt_file::VirtFile};


pub static mut FILE_TREE: Option<Rc<INode>> = None;

#[derive(Clone)]
pub enum DiskFileEnum {
    DiskFile(DiskFile),
    DiskDir(Dir),
    VirtFile(VirtFile),
    VirtDir,
    None
}

// 文件树原始树
pub struct INodeInner {
    pub filename: String,               // 文件名
    pub file_type: FileType,            // 文件数类型
    pub parent: Option<Weak<INode>>,    // 父节点
    pub children: Vec<Rc<INode>>,       // 子节点
    pub file: DiskFileEnum              // 硬盘文件
}

pub struct INode(pub RefCell<INodeInner>);

impl INode {
    // 创建文件 创建文件时需要使用文件名
    pub fn new(filename: String, file: DiskFileEnum, 
            file_type: FileType, parent: Option<Weak<INode>>) -> Rc<Self> {
        Rc::new(Self(RefCell::new(INodeInner {
            filename, 
            file_type, 
            parent, 
            children: vec![],
            file
        })))
    }

    // 根目录节点
    pub fn root() -> Rc<INode> {
        unsafe {
            if let Some(data) = &FILE_TREE {
                return data.clone();
            };
            todo!("无法在为初始化之前调用root")
        }
    }

    // 添加节点到父节点
    pub fn add(self: Rc<Self>, child: Rc<INode>) {
        let mut inner = self.0.borrow_mut();
        let mut cinner = child.0.borrow_mut();
        cinner.parent = Some(Rc::downgrade(&self));
        drop(cinner);
        inner.children.push(child);
    }

    pub fn get_children(self: Rc<Self>, filename: &str) -> Result<Rc<INode>, RuntimeError> {
        match filename {
            "."     => Ok(self.clone()),
            ".."    => {
                let inner = self.0.borrow_mut();
                match inner.parent.clone() {
                    Some(parent) => {
                        match parent.upgrade() {
                            Some(p) => Ok(p.clone()),
                            None => Ok(self.clone())
                        }
                    },
                    None => {
                        Ok(self.clone())
                    }
                }
            },
            _ => {
                for child in self.clone_children() {
                    if child.get_filename() == filename {
                        return Ok(child.clone());
                    }
                }
                Err(RuntimeError::FileNotFound)
            }
        }
    }

    pub fn find(self: Rc<Self>, path: &str) -> Result<Rc<INode>, RuntimeError> {
        // traverse path
        let (name, rest_opt) = get_curr_dir(path);
        if let Some(rest) = rest_opt {
            // 如果是文件夹
            self.get_children(name)?.find(rest)
        } else {
            self.get_children(name)
        }
    }

    // 根据路径 获取文件节点
    pub fn get(current: Option<Rc<INode>>, path: &str) -> Result<Rc<INode>, RuntimeError> {
        if let Some(node) = current {
            node.find(path)
        } else {
            Self::root().find(path)
        }
    }
    // 根据路径 获取文件节点
    pub fn open(current: Option<Rc<INode>>, path: &str) -> Result<Rc<File>, RuntimeError> {
        let inode = Self::get(current, path)?;
        if let Some(file) = get_cache_file(&inode.get_filename()) {
            return Ok(file.clone());
        }
        File::new(inode)
    }
    // 根据路径 获取文件节点
    pub fn open_or_create(current: Option<Rc<INode>>, path: &str) -> Result<Rc<File>, RuntimeError> {
        if let Ok(inode) = Self::get(current.clone(), path) {
            if let Some(file) = get_cache_file(&inode.get_filename()) {
                return Ok(file.clone());
            }
            File::new(inode)
        } else {
            let (dir_path, filename) = split_path(path);
            
            debug!("split path: {:?}  filename: {}", dir_path, filename);

            let dir_inode = 
                dir_path.map_or(Ok(INode::root()), |x| INode::get(current, x))?;

            let file = VirtFile::new(filename.to_string());

            let parent_node = Some(Rc::downgrade(&dir_inode));
            let file_node = INode::new(filename.to_string(), 
            DiskFileEnum::VirtFile(file), FileType::VirtFile, parent_node);
            dir_inode.clone().add(file_node.clone());
            File::new(file_node)
            // Err(RuntimeError::FileNotFound)
        }
    }

    // 获取当前路径
    pub fn get_pwd(&self) -> String {
        let tree_node = self.clone();
        let mut path = String::new();
        loop {
            path = path + "/" + &tree_node.get_filename();
            if self.is_root() { break; }
        }
        path
    }

    // 判断当前是否为根目录
    pub fn is_root(&self) -> bool {
        // 根目录文件名为空
        self.0.borrow().parent.is_none()
    }

    // 判断是否为目录
    pub fn is_dir(&self) -> bool {
        match self.0.borrow().file_type {
            FileType::Directory => true,
            _ => false
        }
    }

    pub fn is_virt_file(&self) -> bool {
        match self.0.borrow().file_type {
            FileType::VirtFile => true,
            _ => false
        }
    }

    // 获取文件名
    pub fn get_filename(&self) -> String{
        self.0.borrow_mut().filename.clone()
    }

    // 获取子元素
    pub fn clone_children(&self) -> Vec<Rc<INode>> {
        self.0.borrow().children.clone()
    }

    // 判断是否为空
    pub fn is_empty(&self) -> bool {
        self.0.borrow_mut().children.is_empty()
    }

    // 删除子节点
    pub fn delete(&self, filename: &str) {
        self.0.borrow_mut().children.retain(|c| c.get_filename() != filename);
    }

    // 获取文件大小
    pub fn get_file_size(&self) -> usize {
        match &self.0.borrow_mut().file {
            DiskFileEnum::DiskFile(f) => f.size().unwrap() as usize,
            _ => 0
        }
    }

    // 获取文件类型
    pub fn get_file_type(&self) -> FileType {
        self.0.borrow_mut().file_type
    }

    // 读取文件内容
    pub fn read(&self) -> Result<Vec<u8>, RuntimeError>{
        let mut file_vec = vec![0u8; self.get_file_size()];
        // self.to_file()?.read_exact(&mut file_vec);
        self.read_to(&mut file_vec)?;
        Ok(file_vec)
    }

    pub fn to_file(&self) -> Result<DiskFile, RuntimeError>{
        if let DiskFileEnum::DiskFile(f) = &self.0.borrow().file {
            Ok(f.clone())
        } else {
            Err(RuntimeError::NotRWFile)
        }
    }
    
    // 读取文件内容
    pub fn read_to(&self, buf: &mut [u8]) -> Result<usize, RuntimeError>  {
        // 不再处理虚拟文件
        // self.0.borrow_mut().file.read_exact(buf);
        // 读取错误 但是会抛出异常 UnexpectedEOF
        // self.to_file()?.read_exact(buf).expect("读取错误");
        let mut file = self.to_file()?;
        file.read_exact(buf);
        Ok(buf.len())
    }

    // 写入设备
    pub fn write(&self, buf: &mut [u8]) -> Result<usize, RuntimeError> {
        // self.0.borrow_mut().file.write(buf).unwrap()
        self.to_file()?.write(buf).map_err(|_| RuntimeError::NotRWFile)
    }

    // 创建文件夹
    // TODO: 创建文件夹
    pub fn mkdir(current: Option<Rc<INode>>, path: &str, _flags: u16) -> Result<Rc<INode>, RuntimeError>{
        match Self::get(current.clone(), path) {
            Ok(inode) => Ok(inode),
            Err(_) => {
                // 创建文件夹
                let (dir, filename) = split_path(path);
                let pnode = match dir {
                    Some(path) => INode::get(current, path)?,
                    None => INode::root()
                };

                let parent_node = Some(Rc::downgrade(&pnode));
                let file_node = INode::new(filename.to_string(), 
                DiskFileEnum::VirtDir, FileType::Directory, parent_node);
                pnode.clone().add(file_node.clone());
                Ok(file_node)
            }
        }
    }

    // 删除自身
    pub fn del_self(&self) {
        let inner = self.0.borrow_mut();
        let parent = inner.parent.clone();
        if let Some(parent) = parent {
            let parent = parent.upgrade().unwrap();
            let filename = inner.filename.clone();
            drop(inner);
            parent.delete(&filename);
        }
    }

    // 删除自身
    pub fn is_valid(&self) -> bool {
        let inner = self.0.borrow_mut();
        let parent = inner.parent.clone();
        if let Some(parent) = parent {
            parent.upgrade().is_some()
        } else {
            false
        }
    }

    // link at
    pub fn linkat(&self, path: &str) {
        let (dir, filename) = split_path(path);
        let pnode = match dir {
            Some(path) => INode::get(None, path).expect("don't hava this folder"),
            None => INode::root()
        };
        let inner = self.0.borrow_mut();
        let new_node = Self::new(filename.to_string(), inner.file.clone(),
            inner.file_type, inner.parent.clone());

        pnode.add(new_node);

        // if let Some(node) = inner.parent.as_ref().map_or(None, |x| x.upgrade()) {
        //     node.add(new_node);
        // }
        // let parent_node = self.0.
    }

}

fn get_curr_dir(path: &str) -> (&str, Option<&str>) {
    let trimmed_path = path.trim_matches('/');
    trimmed_path.find('/').map_or((trimmed_path, None), |n| {
        (&trimmed_path[..n], Some(&trimmed_path[n + 1..]))
    })
}

// spilit_path get dir and filename
fn split_path(path: &str) -> (Option<&str>, &str) {
    let trimmed_path = path.trim_matches('/');
    trimmed_path.rfind('/').map_or((None, trimmed_path), |n| {
        (Some(&trimmed_path[..n]), &trimmed_path[n + 1..])
    })
}

// pub fn mount(path: &str, root_dir: Dir) {
//     for i in root_dir.iter() {
//         let file = i.unwrap();
//         if file.is_dir() {
//             // 如果是文件夹的话进行 深度遍历
//             mount(&(path.to_string() + &file.file_name() + "/"), file.to_dir());
//         } else {
//             // 如果是文件的话则进行挂载
//             INode::new(filename, file, file_type, parent)
//         }
//     }
// }

pub fn add_files_to_dir(dir: Dir, node: Rc<INode>) {
    for file_entry in dir.iter() {
        let file_entry = file_entry.expect("文件节点异常");
        let filename = file_entry.file_name();
        if filename == "." || filename == ".." {
            continue;
        }
        if file_entry.is_dir() {
            let child_dir = file_entry.to_dir();
            let parent_node = Some(Rc::downgrade(&node));
            let dir_node = INode::new(filename, 
                DiskFileEnum::DiskDir(child_dir.clone()), FileType::Directory, parent_node);
            node.clone().add(dir_node.clone());
            add_files_to_dir(child_dir, dir_node);
        } else if file_entry.is_file() {
            let parent_node = Some(Rc::downgrade(&node));
            let dir_node = INode::new(filename, 
        DiskFileEnum::DiskFile(file_entry.to_file()), FileType::File, parent_node);
            node.clone().add(dir_node.clone());
        } else {
            error!("不支持的文件类型");
        }
    }
}

pub fn init(path: &str, root_dir: Dir) {
    if path == "/" {
        let inode = INode::new(String::from(""), DiskFileEnum::DiskDir(root_dir.clone()), 
            FileType::Directory, None);
        // 添加到文件树子节点
        unsafe { FILE_TREE = Some(inode.clone()); }
        add_files_to_dir(root_dir, inode);
    }
}