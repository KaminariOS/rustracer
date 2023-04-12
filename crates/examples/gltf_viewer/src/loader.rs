//! Non-blocking model loader
//!
//! The loader starts a worker thread that will wait for load messages.
//! Once a message is received the thread will load the model and send the
//! loaded model through another channel.
//!
//! When dropping the loader, a stop message is sent to the thread so it can
//! stop listening for load events. Then we wait for the thread to terminate.
//!
//! Users have to call `load` to load a new model and `get_model` to retrieve
//! the loaded model.

use log::{info};

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use app::vulkan::Context;
use asset_loader::{Doc, load_file};
use crate::GltfViewerBuilder;
use crate::gui_state::{Scene, Skybox};

enum Message {
    Load(String),
    Stop,
}

pub struct Loader {
    message_sender: Sender<Message>,
    model_receiver: Receiver<Doc>,
    thread_handle: Option<JoinHandle<()>>,
}

impl Loader {
    pub fn new() -> Self {
        let (message_sender, message_receiver) = mpsc::channel();
        let (model_sender, model_receiver) = mpsc::channel();
        let thread_handle = Some(thread::spawn(move || {
            info!("Starting loader");
            loop {
                let message = message_receiver.recv().expect("Failed to receive a path");
                match message {
                    Message::Load(path) => {
                        info!("Start loading {}", path);
                        let pre_loaded_model = load_file(&path);

                        match pre_loaded_model {
                            Ok(pre_loaded_model) => {
                                info!("Finish loading {}", path);
                                model_sender.send(pre_loaded_model).unwrap();
                            }
                            Err(error) => {
                                log::error!(
                                    "Failed to load {}. Cause: {:?}",
                                    path,
                                    error
                                );
                            }
                        }
                    }
                    Message::Stop => break,
                }
            }
            info!("Stopping loader");
        }));

        Self {
            message_sender,
            model_receiver,
            thread_handle,
        }
    }

    /// Start loading a new model in the background.
    ///
    /// Call `get_model` to retrieve the loaded model.
    pub fn load(&self, path: String) {
        self.message_sender
            .send(Message::Load(path))
            .expect("Failed to send load message to loader");
    }

    /// Get the last loaded model.
    ///
    /// If no model is ready, then `None` is returned.
    pub fn get_model(&self) -> Option<Doc> {
        match self.model_receiver.try_recv() {
            Ok(mut pre_loaded_model) => Some(pre_loaded_model),
            _ => None,
        }
    }
}

// fn pre_load_model<P: AsRef<Path>>(
//     path: P,
// ) -> Result<PreLoadedResource<Model, ModelStagingResources>, Box<dyn Error>> {
//     let device = context.device();
//
//     // Create command buffer
//
// }

impl Drop for Loader {
    fn drop(&mut self) {
        self.message_sender
            .send(Message::Stop)
            .expect("Failed to send stop message to loader thread");
        if let Some(handle) = self.thread_handle.take() {
            handle
                .join()
                .expect("Failed to wait for loader thread termination");
        }
        info!("Loader dropped");
    }
}




