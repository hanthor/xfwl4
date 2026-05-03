// xfwl4 -- Wayland compositor for the Xfce Desktop Environment
//
// Copyright (C) 2026 Brian Tarricone <brian@tarricone.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::sync::{Arc, Mutex};

#[cfg(any(feature = "winit", feature = "udev"))]
use smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_frame_v1::EVT_LINUX_DMABUF_SINCE;
use smithay::{
    backend::allocator::Fourcc,
    output::{Output, WeakOutput},
    reexports::{
        wayland_protocols_wlr::screencopy::v1::server::{
            zwlr_screencopy_frame_v1::{EVT_BUFFER_DONE_SINCE, EVT_DAMAGE_SINCE, Flags, ZwlrScreencopyFrameV1},
            zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            backend::{ClientId, GlobalId},
            protocol::{wl_buffer::WlBuffer, wl_output::WlOutput, wl_shm},
        },
    },
    utils::{Buffer as BufferCoords, Logical, Monotonic, Rectangle, Size, Time},
};

use crate::core::util::OutputExt;

pub struct WlrScreencopyState {
    _global: GlobalId,
    manager_instances: Vec<ZwlrScreencopyManagerV1>,
    frames: Vec<WlrFrameRef>,
}

impl WlrScreencopyState {
    pub fn new<H, F>(dh: &DisplayHandle, filter: F) -> Self
    where
        H: WlrScreencopyHandler,
        F: for<'c> Fn(&'c Client) -> bool + Send + Sync + 'static,
    {
        let global = dh.create_global::<H, ZwlrScreencopyManagerV1, _>(3, Box::new(filter));
        Self {
            _global: global,
            manager_instances: Vec::new(),
            frames: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct WlrFrame(WlrFrameRef);

impl WlrFrame {
    pub fn output_rect(&self) -> Rectangle<i32, Logical> {
        self.0.inner.lock().unwrap().output_rect
    }

    pub fn buffer_size(&self) -> Size<i32, BufferCoords> {
        self.0.inner.lock().unwrap().buffer_size
    }

    pub fn overlay_cursor(&self) -> bool {
        self.0.inner.lock().unwrap().overlay_cursor
    }

    pub fn send_flags(&self, flags: Flags) {
        let inner = self.0.inner.lock().unwrap();
        if !inner.finished {
            self.0.instance.flags(flags);
        }
    }

    pub fn send_damage(&self, damage_rect: Rectangle<i32, BufferCoords>) {
        let inner = self.0.inner.lock().unwrap();
        if !inner.finished && inner.should_send_damage && self.0.instance.version() > EVT_DAMAGE_SINCE {
            self.0.instance.damage(
                damage_rect.loc.x as u32,
                damage_rect.loc.y as u32,
                damage_rect.size.w as u32,
                damage_rect.size.h as u32,
            );
        }
    }

    pub fn send_ready(self, timestamp: Time<Monotonic>) {
        let mut inner = self.0.inner.lock().unwrap();
        if !inner.finished {
            inner.finished = true;
            drop(inner);
            let tv_sec_lo = timestamp.as_millis() / 1000;
            let tv_nsec = (timestamp.as_micros() % 1_000_000) * 1000;
            self.0.instance.ready(0, tv_sec_lo, tv_nsec as u32);
        }
    }

    pub fn send_failed(self) {
        self.send_failed_internal();
    }

    fn send_failed_internal(&self) {
        let mut inner = self.0.inner.lock().unwrap();
        if !inner.finished {
            inner.finished = true;
            drop(inner);
            self.0.instance.failed();
        }
    }
}

impl Drop for WlrFrame {
    fn drop(&mut self) {
        self.send_failed_internal();
    }
}

#[derive(Debug, Clone)]
pub struct WlrFrameRef {
    instance: ZwlrScreencopyFrameV1,
    inner: Arc<Mutex<WlrFrameInner>>,
}

impl PartialEq for WlrFrameRef {
    fn eq(&self, other: &Self) -> bool {
        self.instance == other.instance
    }
}

pub struct WlrBufferConstraints {
    /// Required buffer size
    pub size: Size<i32, BufferCoords>,
    /// Available SHM formats, and rowstride for that format
    pub shm: Vec<(wl_shm::Format, u32)>,
    #[cfg(any(feature = "winit", feature = "x11", feature = "udev"))]
    /// Available linux-dmabuf formats, if any
    pub dma: Vec<Fourcc>,
}

#[derive(Debug)]
struct WlrFrameInner {
    output: WeakOutput,
    output_rect: Rectangle<i32, Logical>,
    buffer_size: Size<i32, BufferCoords>,
    overlay_cursor: bool,
    should_send_damage: bool,
    finished: bool,
}

pub trait WlrScreencopyHandler
where
    Self: GlobalDispatch<ZwlrScreencopyManagerV1, Box<dyn for<'c> Fn(&'c Client) -> bool + Send + Sync>>
        + Dispatch<ZwlrScreencopyManagerV1, ()>
        + Dispatch<ZwlrScreencopyFrameV1, ()>
        + Sized
        + 'static,
{
    fn wlr_screencopy_state(&mut self) -> &mut WlrScreencopyState;

    fn buffer_constraints(&mut self, output: &Output, output_rect: Rectangle<i32, Logical>) -> Option<WlrBufferConstraints>;

    fn on_copy(&mut self, frame: WlrFrame, output: Output, buffer: WlBuffer);
}

impl<H: WlrScreencopyHandler> GlobalDispatch<ZwlrScreencopyManagerV1, Box<dyn for<'c> Fn(&'c Client) -> bool + Send + Sync>, H>
    for WlrScreencopyState
{
    fn bind(
        state: &mut H,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrScreencopyManagerV1>,
        _global_data: &Box<dyn for<'c> Fn(&'c Client) -> bool + Send + Sync>,
        data_init: &mut DataInit<'_, H>,
    ) {
        let instance = data_init.init(resource, ());
        state.wlr_screencopy_state().manager_instances.push(instance);
    }

    fn can_view(client: Client, global_data: &Box<dyn for<'c> Fn(&'c Client) -> bool + Send + Sync>) -> bool {
        global_data(&client)
    }
}

impl<H: WlrScreencopyHandler> Dispatch<ZwlrScreencopyManagerV1, (), H> for WlrScreencopyState {
    fn request(
        state: &mut H,
        client: &Client,
        resource: &ZwlrScreencopyManagerV1,
        request: <ZwlrScreencopyManagerV1 as Resource>::Request,
        data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, H>,
    ) {
        use smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_manager_v1::Request;

        match request {
            Request::CaptureOutput {
                frame,
                overlay_cursor,
                output,
            } => {
                let instance = data_init.init(frame, ());
                init_frame(state, instance, overlay_cursor, output, None);
            }

            Request::CaptureOutputRegion {
                frame,
                overlay_cursor,
                output,
                x,
                y,
                width,
                height,
            } => {
                let instance = data_init.init(frame, ());
                init_frame(
                    state,
                    instance,
                    overlay_cursor,
                    output,
                    Some(Rectangle::new((x, y).into(), (width, height).into())),
                );
            }

            Request::Destroy => {
                <Self as Dispatch<ZwlrScreencopyManagerV1, (), H>>::destroyed(state, client.id(), resource, data);
            }

            _ => (),
        }
    }

    fn destroyed(state: &mut H, _client: ClientId, resource: &ZwlrScreencopyManagerV1, _data: &()) {
        state
            .wlr_screencopy_state()
            .manager_instances
            .retain(|instance| instance != resource);
    }
}

impl<H: WlrScreencopyHandler> Dispatch<ZwlrScreencopyFrameV1, (), H> for WlrScreencopyState {
    fn request(
        state: &mut H,
        client: &Client,
        resource: &ZwlrScreencopyFrameV1,
        request: <ZwlrScreencopyFrameV1 as Resource>::Request,
        data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, H>,
    ) {
        use smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_frame_v1::Request;

        if let Some(frame) = state
            .wlr_screencopy_state()
            .frames
            .iter()
            .find(|frame| &frame.instance == resource)
            .cloned()
        {
            match request {
                Request::Copy { buffer } => {
                    handle_copy(state, frame, buffer);
                }

                Request::CopyWithDamage { buffer } => {
                    frame.inner.lock().unwrap().should_send_damage = true;
                    handle_copy(state, frame, buffer);
                }

                Request::Destroy => {
                    <Self as Dispatch<ZwlrScreencopyFrameV1, (), H>>::destroyed(state, client.id(), resource, data);
                }

                _ => (),
            }
        }
    }

    fn destroyed(state: &mut H, _client: ClientId, resource: &ZwlrScreencopyFrameV1, _data: &()) {
        state.wlr_screencopy_state().frames.retain(|frame| &frame.instance != resource);
    }
}

fn output_rect_for_frame(output: &Output, capture_rect: Option<Rectangle<i32, Logical>>) -> Option<Rectangle<i32, Logical>> {
    output.geometry().and_then(|output_geom| {
        capture_rect
            .map(|mut rect| {
                rect.loc += output_geom.loc;
                output_geom.intersection(rect)
            })
            .unwrap_or_else(|| Some(output_geom))
            .map(|mut output_rect| {
                output_rect.loc -= output_geom.loc;
                output_rect
            })
    })
}

fn init_frame<H: WlrScreencopyHandler>(
    state: &mut H,
    instance: ZwlrScreencopyFrameV1,
    overlay_cursor: i32,
    output: WlOutput,
    capture_rect: Option<Rectangle<i32, Logical>>,
) {
    if let Some(output) = Output::from_resource(&output)
        && let Some(output_rect) = output_rect_for_frame(&output, capture_rect)
        && let Some(constraints) = state.buffer_constraints(&output, output_rect)
    {
        let frame = WlrFrameRef {
            instance: instance.clone(),
            inner: Arc::new(Mutex::new(WlrFrameInner {
                overlay_cursor: overlay_cursor != 0,
                output: output.downgrade(),
                output_rect,
                buffer_size: constraints.size,
                should_send_damage: false,
                finished: false,
            })),
        };
        state.wlr_screencopy_state().frames.push(frame);

        for (format, stride) in constraints.shm {
            instance.buffer(format, constraints.size.w as u32, constraints.size.h as u32, stride);
        }

        #[cfg(any(feature = "winit", feature = "udev"))]
        if instance.version() >= EVT_LINUX_DMABUF_SINCE {
            for format in constraints.dma {
                instance.linux_dmabuf(format as u32, constraints.size.w as u32, constraints.size.h as u32);
            }
        }

        if instance.version() >= EVT_BUFFER_DONE_SINCE {
            instance.buffer_done();
        }
    } else {
        instance.failed();
    }
}

fn handle_copy<H: WlrScreencopyHandler>(state: &mut H, frame: WlrFrameRef, buffer: WlBuffer) {
    let frame = WlrFrame(frame);
    let output = frame.0.inner.lock().unwrap().output.upgrade();
    if let Some(output) = output {
        state.on_copy(frame, output, buffer);
    } else {
        frame.send_failed();
    }
}

macro_rules! delegate_wlr_screencopy {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1: Box<dyn for<'c> Fn(&'c smithay::reexports::wayland_server::Client) -> bool + Send + Sync>
        ] => $crate::protocols::wlr_screencopy::WlrScreencopyState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1: ()
        ] => $crate::protocols::wlr_screencopy::WlrScreencopyState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1: ()
        ] => $crate::protocols::wlr_screencopy::WlrScreencopyState);
    };
}

pub(crate) use delegate_wlr_screencopy;
