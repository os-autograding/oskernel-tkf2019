use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::marker::Sync;
use core::ops::{Drop, Deref, DerefMut};
use core::option::Option::{self, None, Some};
use core::default::Default;
use core::hint::spin_loop;


// 在编译时已经确定大小
// 所有参数都必须实现了Sized绑定
// 特殊语法是？Sized表示如果绑定不适合使用将会移除
pub struct Mutex<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Mutex<T> {
        Mutex {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    #[allow(unused)]
    pub fn into_runner(self) -> T {
        // 注意data的变量名一定要跟Mutex中的成员名一致
        // 这里只获取Mutex.data
        let Mutex { data, .. } = self;
        data.into_inner()
    }
}


unsafe impl<T: ?Sized> Sync for Mutex<T> {}

unsafe impl<T: ?Sized> Send for Mutex<T> {}

impl<T: ?Sized> Mutex<T> {
    fn obtain_lock(&self) {
        // 尝试获得锁
        loop {
            if let Ok(_) = self.lock.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed) {
                break;
            }
            // 循环判断是否已经解锁如果没有解锁
            while self.lock.load(Ordering::Relaxed) {
                // 向处理器发出信号，表明现在处于自旋状态
                spin_loop();
            }
        }
        // 跳出循环后表明获得锁
    }

    // 锁定
    pub fn lock(&self) -> MutexGuard<T> {
        self.obtain_lock();
        MutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[allow(unused)]
    // 强制解锁
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release)
    }

    #[allow(unused)]
    // 强制获取数据
    pub fn force_get(&self)->MutexGuard<T> {
        MutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }


    #[allow(unused)]
    // 尝试锁定
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        if let Ok(res) = self.lock.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed) {
            return Some(MutexGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            });
        }
        // 循环判断是否已经解锁如果没有解锁
        None
    }
}


impl<T: Sized + Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}


impl<'a, T: Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { &mut *self.data }
}


impl<'a, T: Sized> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.data
    }

}


impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}