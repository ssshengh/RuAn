#![allow(non_snake_case)]

use std::ffi::c_void;
use jni::{
    errors::jni_error_code_to_result,
    objects::{GlobalRef, JClass, JObject},
    strings::JNIString,
    sys::jvalue,
    {sys, JNIEnv, JavaVM},
};
use jni::objects::JValue;
use jni::sys::{jbyte, jclass, jstring};
use log::{debug, error, info, Level, trace, warn};
use crate::util::exec_callback;

/// Java 侧 native 方法名，该方法是打印日志的实际执行方法
const ON_LOG_METHOD_NAME: &str = "onLog";

/// Java 侧，native 方法参数，这里是：
/// * 输入：String，Byte，String
/// * 输出：void
const ON_LOG_METHOD_SIGNATURE: &str = "(Ljava/lang/String;BLjava/lang/String;)V";

/// 安卓注入 SDK 层面的 log 函数抽象
pub struct LogCallbackCtx {
    /// JVM
    jvm: JavaVM,
    cb_ref: GlobalRef,
}

impl LogCallbackCtx {
    pub fn new(env: &JNIEnv, cb_obj: JObject) -> anyhow::Result<LogCallbackCtx> {
        let jvm = env.get_java_vm()?;
        let cb_ref = env.new_global_ref(cb_obj)?;

        Ok(LogCallbackCtx { jvm, cb_ref })
    }

    /// 这个是正常通过 jni 调用 Java 方法时的做法
    /// 该方法主要是 FFI 层调用安卓注册进来的 callback 以打印日志
    #[inline(always)]
    pub unsafe fn on_log_message_impl(
        &self,
        category: &str,
        level: u8,
        message: &str,
    ) -> anyhow::Result<()> {
        exec_callback(move || {
            let mut env = self.jvm.get_env()?;
            let cb_obj = self.cb_ref.as_obj();

            let category = JObject::from(env.new_string(category)?);
            let message = JObject::from(env.new_string(message)?);

            let _ = env.call_method(
                cb_obj,
                ON_LOG_METHOD_NAME,
                ON_LOG_METHOD_SIGNATURE,
                &[
                    (&category).into(),
                    (level as jbyte).into(),
                    (&message).into()
                ],
            )?;

            Ok(())
        });

        Ok(())
    }


    /// 注意，这个方法是一个展示内部宏展开后，Jni 工作流程的一个例子
    ///
    /// 转换 Rust 的日志内容到 Java 层，并调用 logcat 进行打印
    #[inline(always)]
    pub unsafe fn on_log_message_impl_with_no_macro(
        &self,
        category: &str,
        level: u8,
        message: &str,
    ) -> anyhow::Result<()> {
        // 1. 获取 JVM
        let jvm_ptr = self.jvm.get_java_vm_pointer();

        // 2. 获取 ENV
        let mut ptr = std::ptr::null_mut();
        self.get_env(jvm_ptr, ptr)?;
        let env_ptr = ptr as *mut sys::JNIEnv;

        // 3. 获取注册进来用于给到安卓 logcat 打印的对象指针
        let cb_obj_ptr = self.cb_ref.as_obj().as_raw();
        if cb_obj_ptr.is_null() {
            anyhow::bail!("[jni] cb_ref can not be null")
        }

        // 4. 获取对象，用于获取方法
        #[allow(unused_assignments)]
            let mut cb_obj_class: jclass = std::ptr::null_mut();
        if let Some(f) = (*(*env_ptr)).GetObjectClass {
            cb_obj_class = f(env_ptr, cb_obj_ptr)
        } else {
            anyhow::bail!("[jni-rust] [log] GetObjectClass not found on env")
        }

        // 5. 获取 Java 方法 ID
        #[allow(unused_assignments)]
            let mut cb_method_id = std::ptr::null_mut();
        if let Some(f) = (*(*env_ptr)).GetMethodID {
            // https://docs.oracle.com/en/java/javase/12/docs/specs/jni/types.html#type-signatures
            let name_jni_string = JNIString::from(ON_LOG_METHOD_NAME);
            let sig_jni_string = JNIString::from(ON_LOG_METHOD_SIGNATURE);

            cb_method_id = f(
                env_ptr,
                cb_obj_class,
                name_jni_string.as_ptr(),
                sig_jni_string.as_ptr(),
            )
        } else {
            anyhow::bail!("[jni-rust] [log] GetMethodID not found on env")
        }

        // 6. 新建一个 Java 指针，转换传输给 Java native 方法的参数
        #[allow(unused_assignments)]
            let mut category_ptr = std::ptr::null_mut();
        #[allow(unused_assignments)]
            let mut message_ptr = std::ptr::null_mut();
        if let Some(f) = (*(*env_ptr)).NewStringUTF {
            let category_jni_string = JNIString::from(category);
            let message_jni_string = JNIString::from(message);

            category_ptr = f(env_ptr, category_jni_string.as_ptr());
            message_ptr = f(env_ptr, message_jni_string.as_ptr());
        } else {
            anyhow::bail!("[jni-rust] [log] NewStringUTF not found on env")
        }

        // 7. 调用打印到 logcat 的方法，这里使用 A 方式：https://docs.oracle.com/en/java/javase/12/docs/specs/jni/functions.html#calltypemethod-routines-calltypemethoda-routines-calltypemethodv-routines
        if let Some(f) = (*(*env_ptr)).CallVoidMethodA {
            let args = vec![
                jvalue { l: category_ptr },
                jvalue { b: level as _ },
                jvalue { l: message_ptr },
            ];
            f(env_ptr, cb_obj_ptr, cb_method_id, args.as_ptr())
        } else {
            anyhow::bail!("[jni-rust] [log] CallVoidMethodA not found on env")
        }

        Ok(())
    }

    #[inline(always)]
    pub fn level_to_u8(level: Level) -> u8 {
        match level {
            Level::Error => 1,
            Level::Warn => 2,
            Level::Info => 3,
            Level::Debug => 4,
            Level::Trace => 5,
        }
    }

    /// 获取 JVM 环境方法：
    /// * self.jvm.get_env()
    /// 宏展开后的内部实现
    unsafe fn get_env(&self, jvm_ptr: *mut sys::JavaVM, mut ptr: *mut c_void) -> anyhow::Result<()>{
        // 尝试获取 Env：https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html#getenv
        // 首先是查看是否有 GetEnv 方法，其次是看是否能够正确获取到 Env
        if let Some(fun) = (*(*jvm_ptr)).GetEnv {
            let ret = fun(jvm_ptr, &mut ptr, sys::JNI_VERSION_1_1);
            jni_error_code_to_result(ret)?;
        } else {
            anyhow::bail!("[jni] GetEnv not found on jvm")
        }

        Ok(())
    }
}

impl Drop for LogCallbackCtx {
    fn drop(&mut self) {
        log::info!("[drop] LogCallbackCtx is dropped!")
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_ss_ruan_sdkbridge_SdkLogger_initLog(
    env: JNIEnv,
    _: JClass,
    log_cb: JObject,
) {
    match LogCallbackCtx::new(&env, log_cb) {
        Ok(ctx) => {
            // WARN: 闭包的所有调用都不应含有 log::xxx!() 方式输出日志, 会导致递归调用
            let log_callback = Box::new(move |category: &str, level: log::Level, message: &str| {
                let level = LogCallbackCtx::level_to_u8(level);
                if let Err(e) = ctx.on_log_message_impl(category, level, message) {
                    eprintln!("[jni] on_log_message failed: {}", e)
                }
            });

            if let Err(e) = logging::init(Some(log_callback)) {
                error!("[jni] initLog failed: {:?}", e)
            }

            info!("[jni] initLog set panic hook");
            std::panic::set_hook(Box::new(panic_hook));

            info!("[jni] initLog succeed");
            // warn!("[jni] initLog succeed");
            // error!("[jni] just test initLog succeed");
            // debug!("[jni] initLog succeed");
            // trace!("[jni] initLog succeed")
        }
        Err(e) => {
            eprintln!("[jni] LogCallbackCtx::new failed: {}", e)
        }
    }
}

/// 设置 panic 回调，次回调会覆盖 Rust 原有的处理
fn panic_hook(info: &std::panic::PanicInfo) {
    log::error!("sdk-panic {:?}", info);
}
