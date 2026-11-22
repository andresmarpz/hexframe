/******************************************************************************
    Copyright (C) 2023 by Lain Bailey <lain@obsproject.com>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 2 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
******************************************************************************/

use std::ffi::c_void;
use std::mem::size_of;

use greyscreen_core::health::gpu::{GpuScheduling, HagsSupport};
use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_CLOSEADAPTER, D3DKMT_OPENADAPTERFROMLUID, D3DKMT_QUERYADAPTERINFO, D3DKMT_WDDM_2_7_CAPS,
    D3DKMT_WDDM_2_9_CAPS, D3DKMTCloseAdapter, D3DKMTOpenAdapterFromLuid, D3DKMTQueryAdapterInfo,
    KMTQAITYPE_WDDM_2_7_CAPS, KMTQAITYPE_WDDM_2_9_CAPS, KMTQUERYADAPTERINFOTYPE,
};
use windows::Win32::Foundation::{LUID, NTSTATUS};

const SUPPORTED_BIT: u32 = 1;
const ENABLED_BIT: u32 = 2;
const SUPPORT_STATE_MASK: u32 = 3;

pub(crate) fn query(luid: LUID) -> Option<GpuScheduling> {
    let mut open = D3DKMT_OPENADAPTERFROMLUID {
        AdapterLuid: luid,
        ..Default::default()
    };
    // SAFETY: `open` is a live struct with the adapter id set and the call only writes its handle.
    let status = unsafe { D3DKMTOpenAdapterFromLuid(&mut open) };
    if status.is_err() {
        tracing::debug!(status = status.0, "open kernel adapter failed");
        return None;
    }
    let scheduling = read(open.hAdapter);
    let close = D3DKMT_CLOSEADAPTER {
        hAdapter: open.hAdapter,
    };
    // SAFETY: `close` holds the handle opened above and it is closed once.
    let status = unsafe { D3DKMTCloseAdapter(&close) };
    if status.is_err() {
        tracing::debug!(status = status.0, "close kernel adapter failed");
    }
    scheduling
}

fn read(handle: u32) -> Option<GpuScheduling> {
    let mut caps = D3DKMT_WDDM_2_7_CAPS::default();
    let status = query_caps(handle, KMTQAITYPE_WDDM_2_7_CAPS, &mut caps);
    if status.is_err() {
        tracing::warn!(status = status.0, "query of wddm 2.7 caps failed");
        return None;
    }
    // SAFETY: the union holds plain integer data that the driver filled.
    let bits = unsafe { caps.Anonymous.Value };
    let mut support = if bits & SUPPORTED_BIT != 0 {
        HagsSupport::Stable
    } else {
        HagsSupport::AlwaysOff
    };
    let mut extended = D3DKMT_WDDM_2_9_CAPS::default();
    if query_caps(handle, KMTQAITYPE_WDDM_2_9_CAPS, &mut extended).is_ok() {
        // SAFETY: as above.
        let state = unsafe { extended.Anonymous.Value } & SUPPORT_STATE_MASK;
        support = HagsSupport::from_state(state);
    }
    Some(GpuScheduling {
        enabled: bits & ENABLED_BIT != 0,
        support,
    })
}

fn query_caps<T>(handle: u32, kind: KMTQUERYADAPTERINFOTYPE, caps: &mut T) -> NTSTATUS {
    let mut args = D3DKMT_QUERYADAPTERINFO {
        hAdapter: handle,
        Type: kind,
        pPrivateDriverData: std::ptr::from_mut(caps).cast::<c_void>(),
        PrivateDriverDataSize: size_of::<T>() as u32,
    };
    // SAFETY: the data pointer addresses a live caps struct of the stated size.
    unsafe { D3DKMTQueryAdapterInfo(&mut args) }
}
