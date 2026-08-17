use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::time::Duration;

use web_time::Instant;

use bluetooth_driver::driver::{
    Adapter, Address, BluetoothDriver, Device, DeviceInfo, DriverError, DriverEvent, PnpId,
    VendorIdSource,
};

use crate::key::Key;

/// The device type a given driver's adapters hand out - spelled out once
/// here since `<D::Adapter as Adapter>::Device` gets verbose fast.
type DeviceOf<D> = <<D as BluetoothDriver>::Adapter as Adapter>::Device;

const BATTERY_REFRESH_INTERVAL: Duration = Duration::from_secs(10);
const FULL_INFO_REFRESH_INTERVAL: Duration = Duration::from_secs(15);

/// Bounds silent, periodic background calls (device list / battery / full
/// info / adapter refresh) far tighter than the driver's own 30s
/// connection-level timeout. Those run every tick with nobody watching,
/// so stalling the whole UI for 30s before the "BlueZ not responding"
/// badge even shows up reads as a hang, not a slow response. User-
/// initiated actions like pair/connect deliberately skip this and keep
/// the driver's longer timeout instead - the user pressed a key and is
/// already watching, and a real pairing flow can legitimately need to
/// wait on a passkey confirmation elsewhere.
const BACKGROUND_CALL_TIMEOUT: Duration = Duration::from_secs(5);

const MAX_LOGGED_EVENTS: usize = 5_000;
const STATUS_BANNER_LIFETIME: Duration = Duration::from_secs(6);

/// Which top-level screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Adapters + device list + detail pane.
    Shell,
    /// Live scan results.
    Discovery,
    /// The driver event log.
    EventLog,
    /// Power/discoverable/pairable controls for the current adapter.
    AdapterControl,
}

/// Which modal, if any, is currently drawn on top of the active screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// No overlay open.
    None,
    /// The vendor-attribution chain for the selected device.
    Vendor,
    /// Editing the selected device's alias.
    AliasEdit,
    /// The keymap reference.
    Help,
    /// The `:`-triggered command palette.
    Palette,
    /// "Forget this device?" confirmation before `App::forget_selected`.
    ConfirmForget,
}

/// Which column has keyboard focus on the Shell screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The adapters column.
    Adapters,
    /// The device list column.
    Devices,
    /// The detail pane.
    Detail,
}

/// Which subset of `App::devices` is shown, cycled with `/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFilter {
    /// Every known device.
    All,
    /// Only paired devices.
    Paired,
    /// Only currently-connected devices.
    Connected,
    /// Only unpaired ("nearby") devices - the Discovery screen's default.
    Nearby,
    /// Only blocked devices.
    Blocked,
}

impl DeviceFilter {
    /// The filter's label as shown in the filter bar (e.g. `"PAIRED"`).
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Paired => "PAIRED",
            Self::Connected => "CONNECTED",
            Self::Nearby => "NEARBY",
            Self::Blocked => "BLOCKED",
        }
    }

    /// Whether `device` belongs in this filter's subset.
    pub fn matches<Dev: Device>(self, device: &Dev) -> bool {
        match self {
            Self::All => true,
            Self::Paired => device.is_paired(),
            Self::Connected => device.is_connected(),
            Self::Nearby => !device.is_paired(),
            Self::Blocked => device.is_blocked(),
        }
    }

    fn next(self) -> Self {
        match self {
            Self::All => Self::Paired,
            Self::Paired => Self::Connected,
            Self::Connected => Self::Nearby,
            Self::Nearby => Self::Blocked,
            Self::Blocked => Self::All,
        }
    }
}

/// Whether a `StatusBanner` reports success or failure - drives its color.
pub enum BannerKind {
    /// A successful action.
    Info,
    /// A failed action.
    Error,
}

/// A transient banner shown after an action, auto-dismissed after
/// `STATUS_BANNER_LIFETIME`.
pub struct StatusBanner {
    /// Success or failure.
    pub kind: BannerKind,
    /// Short heading, e.g. `"CONNECT FAILED"`.
    pub title: String,
    /// Detail text, typically a `DriverError`'s message.
    pub body: String,
    /// When the banner was shown, for the auto-dismiss timer.
    pub shown_at: Instant,
}

/// One entry in the event log.
pub struct LoggedEvent {
    /// When this app observed the event.
    pub at: Instant,
    /// The event itself.
    pub event: DriverEvent,
}

/// One tier of the vendor-attribution chain, as shown in the vendor
/// modal - each tier states plainly what it actually proves.
pub struct VendorTier {
    /// This tier's name, e.g. `"TIER 1 · GATT PnP ID"`.
    pub label: &'static str,
    /// What this tier's result does or doesn't prove.
    pub caveat: &'static str,
    /// The resolved vendor name, if this tier found one.
    pub result: Option<String>,
    /// How many of the confidence dots to fill (0-3).
    pub confidence: u8, // 0-3 dots
}

/// The full vendor-attribution chain for one device, across all tiers.
pub struct VendorAttribution {
    /// The best available answer across every tier, highest-confidence first.
    pub resolved: Option<String>,
    /// Each tier's own attempt, in display order.
    pub tiers: Vec<VendorTier>,
}

/// A quick, synchronous-only vendor guess for list rows (OUI + beacon
/// company ID) - the full chain, including a live GATT PnP ID read, is
/// only worth the round trip when the user actually opens the vendor
/// modal for one specific device.
pub fn quick_vendor_label<Dev: Device>(device: &Dev) -> Option<String> {
    let address = device.address();
    if let Some(vendor) = bluetooth_driver::oui::vendor(&address) {
        return Some(vendor.to_owned());
    }
    if let Some(&(id, _)) = device.manufacturer_data().first() {
        return Some(match bluetooth_driver::company_id::vendor(id) {
            Some(name) => name.to_owned(),
            None => format!("0x{id:04X}"),
        });
    }
    None
}

impl VendorAttribution {
    /// Runs the full three-tier vendor lookup for `device`, including a
    /// live GATT PnP ID read - only worth the round trip when the user
    /// actually opens the vendor modal for this one device.
    pub async fn compute<Dev: Device>(device: &Dev) -> Self {
        let address = device.address();

        let oui = bluetooth_driver::oui::vendor(&address).map(str::to_owned);
        let oui_caveat = if oui.is_some() {
            "Identifies who registered the radio chip's address block."
        } else if bluetooth_driver::oui::is_locally_administered(&address) {
            "Address is randomized for privacy; carries no manufacturer block."
        } else {
            "No IEEE OUI match for this address."
        };

        let pnp = device.pnp_id().await.ok().flatten();
        let pnp_result = pnp.as_ref().map(|pnp_id| {
            let vendor = match pnp_id.vendor_id_source {
                VendorIdSource::BluetoothSig => bluetooth_driver::company_id::vendor(pnp_id.vendor_id),
                VendorIdSource::Usb => bluetooth_driver::usb_vendor::vendor(pnp_id.vendor_id),
            };
            match vendor {
                Some(name) => name.to_owned(),
                None => format!("0x{:04X} (unresolved)", pnp_id.vendor_id),
            }
        });
        let pnp_caveat = if pnp_result.is_some() {
            "The device states its own vendor ID. Equivalent to a USB VID."
        } else if !device.is_connected() {
            "Not connected. Connect to read the device's own vendor ID."
        } else {
            "Connected, but no PnP ID characteristic exposed."
        };

        let beacon_id = device.manufacturer_data().first().map(|&(id, _)| id);
        let beacon_result = beacon_id.map(|id| match bluetooth_driver::company_id::vendor(id) {
            Some(name) => name.to_owned(),
            None => format!("0x{id:04X} (unresolved)"),
        });
        let beacon_caveat =
            "Proves only that the beacon carries this company's payload, not who built the device.";

        let resolved = pnp_result.clone().or_else(|| oui.clone());

        let tiers = vec![
            VendorTier {
                label: "TIER 1 · GATT PnP ID",
                caveat: pnp_caveat,
                confidence: if pnp_result.is_some() { 3 } else { 0 },
                result: pnp_result,
            },
            VendorTier {
                label: "TIER 2 · IEEE OUI",
                caveat: oui_caveat,
                confidence: if oui.is_some() { 2 } else { 0 },
                result: oui,
            },
            VendorTier {
                label: "TIER 3 · SIG COMPANY ID (beacon)",
                caveat: beacon_caveat,
                confidence: if beacon_result.is_some() { 1 } else { 0 },
                result: beacon_result,
            },
        ];

        Self { resolved, tiers }
    }
}

/// The whole application's state, generic over any `BluetoothDriver`
/// backend. Owned outright by whichever frontend's run loop is
/// driving it; every screen in `ui/*.rs` renders from a `&App<D>`.
pub struct App<D: BluetoothDriver> {
    /// The backend this app is driving.
    pub driver: D,
    /// Every local adapter the driver reported.
    pub adapters: Vec<D::Adapter>,
    /// Index into `adapters` of the currently-selected one.
    pub adapter_idx: usize,
    /// The current adapter's devices.
    pub devices: Vec<DeviceOf<D>>,
    /// Which subset of `devices` is currently shown.
    pub device_filter: DeviceFilter,
    /// Which device is selected, tracked by address rather than position.
    /// `visible_device_indices()` re-sorts by RSSI on every call, and
    /// RSSI keeps changing during an active scan - a positional index
    /// would silently point at a different device every time the list
    /// reshuffles or refreshes.
    pub selected_address: Option<Address>,
    /// Which column has keyboard focus on the Shell screen.
    pub focus: Focus,
    /// Whether the Shell screen's detail pane is expanded fullscreen.
    pub fullscreen_detail: bool,
    /// Which top-level screen is showing.
    pub screen: Screen,
    /// Which modal, if any, is drawn on top of `screen`.
    pub overlay: Overlay,
    /// Live text buffer for the alias-edit overlay.
    pub alias_buffer: String,
    /// Live text buffer for the command palette.
    pub palette_buffer: String,
    /// Every driver event observed so far, oldest first.
    pub events: VecDeque<LoggedEvent>,
    /// How many logged events haven't been viewed on the Events screen yet.
    pub unseen_events: usize,
    /// A transient success/failure banner, if one's currently showing.
    pub status: Option<StatusBanner>,
    /// When the current scan started, for the elapsed-time display.
    pub scan_started_at: Option<Instant>,
    /// The selected device's vendor-attribution chain, once computed
    /// for the vendor overlay.
    pub vendor_info: Option<VendorAttribution>,
    /// Battery level cache for whichever device is currently selected -
    /// a live GATT read, so it's fetched on selection change / a slow
    /// interval rather than every render pass.
    pub battery: Option<u8>,
    battery_for: Option<Address>,
    battery_checked_at: Option<Instant>,
    /// GATT Device Information + PnP ID, fetched only while the
    /// fullscreen detail view is open (it's not needed anywhere else).
    pub full_info: Option<FullDeviceInfo>,
    full_info_for: Option<Address>,
    full_info_checked_at: Option<Instant>,
    /// Client-side "last seen" tracking: BlueZ has no such property, so
    /// this is a timestamp of the last DeviceFound/DeviceUpdated event
    /// per address, on the current adapter only.
    pub last_seen: HashMap<Address, Instant>,
    /// Set the first time a D-Bus call times out (bluetoothd not
    /// answering - a restart is the common case) and cleared the next
    /// time any call succeeds; drives the header's warning badge.
    pub bluez_unresponsive_since: Option<Instant>,
    /// Whether `devices` needs re-fetching on the next tick.
    pub devices_dirty: bool,
    /// Set when the user has asked to quit; the run loop exits once true.
    pub should_quit: bool,
}

/// The selected device's GATT Device Information + PnP ID, as shown on
/// the fullscreen detail view.
pub struct FullDeviceInfo {
    /// Manufacturer/model/firmware text fields.
    pub device_info: DeviceInfo,
    /// The parsed PnP ID characteristic, if the device exposes one.
    pub pnp_id: Option<PnpId>,
}

impl<D: BluetoothDriver> App<D> {
    /// Builds the initial app state: lists adapters, then the first
    /// adapter's devices. Tolerant of a slow/unresponsive backend at
    /// startup (see the `background_timeout` calls below) - this
    /// resolves promptly either way rather than hanging construction.
    pub async fn new(driver: D) -> Result<Self, DriverError> {
        // Neither call can be allowed to hang construction forever or
        // abort it outright: a slow/unresponsive backend at startup
        // (BlueZ restarting, or a browser whose Web Bluetooth
        // getDevices() never resolves without a real backing
        // implementation - observed in an automated/headless browser
        // during this project's own testing) would otherwise leave the
        // app stuck with nothing ever rendered and no visible error.
        // Fall back to empty and let the normal tick-driven retry path
        // (devices_dirty) pick it up once the backend answers.
        let adapters = background_timeout(driver.adapters()).await.unwrap_or_default();
        let (devices, initial_timeout) = match adapters.first() {
            Some(adapter) => match background_timeout(adapter.devices()).await {
                Ok(devices) => (devices, false),
                Err(_) => (Vec::new(), true),
            },
            None => (Vec::new(), false),
        };
        // If discovery was already running before this app started (e.g.
        // left on from bluetoothctl), we don't know the true start time -
        // count from now rather than showing a permanently-stuck 00:00.
        let scan_started_at = adapters
            .first()
            .filter(|a| a.is_discovering())
            .map(|_| Instant::now());

        let mut app = Self {
            driver,
            adapters,
            adapter_idx: 0,
            devices,
            device_filter: DeviceFilter::All,
            selected_address: None,
            focus: Focus::Devices,
            fullscreen_detail: false,
            screen: Screen::Shell,
            overlay: Overlay::None,
            alias_buffer: String::new(),
            palette_buffer: String::new(),
            events: VecDeque::new(),
            unseen_events: 0,
            status: None,
            scan_started_at,
            vendor_info: None,
            battery: None,
            battery_for: None,
            battery_checked_at: None,
            full_info: None,
            full_info_for: None,
            full_info_checked_at: None,
            last_seen: HashMap::new(),
            bluez_unresponsive_since: None,
            devices_dirty: initial_timeout,
            should_quit: false,
        };
        if initial_timeout {
            app.note_bluez_timeout();
        }
        app.normalize_selection();
        Ok(app)
    }

    /// The currently-selected adapter, if `adapters` isn't empty.
    pub fn current_adapter(&self) -> Option<&D::Adapter> {
        self.adapters.get(self.adapter_idx)
    }

    /// Indices into `devices` matching `device_filter`, sorted by RSSI
    /// descending (strongest signal first; devices with no RSSI sort last).
    pub fn visible_device_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.device_filter.matches(*d))
            .map(|(i, _)| i)
            .collect();
        indices.sort_by_key(|&i| {
            std::cmp::Reverse(self.devices[i].rssi().map(|r| r.0).unwrap_or(i16::MIN))
        });
        indices
    }

    /// Position of `selected_address` within `visible_device_indices()`,
    /// if it's currently visible under the active filter.
    pub fn selected_visible_position(&self) -> Option<usize> {
        let address = self.selected_address.clone()?;
        self.visible_device_indices()
            .iter()
            .position(|&i| self.devices[i].address() == address)
    }

    /// The device at `selected_address`, if it's still known.
    pub fn selected_device(&self) -> Option<&DeviceOf<D>> {
        let address = self.selected_address.clone()?;
        self.devices.iter().find(|d| d.address() == address)
    }

    fn selected_device_mut(&mut self) -> Option<&mut DeviceOf<D>> {
        let address = self.selected_address.clone()?;
        self.devices.iter_mut().find(|d| d.address() == address)
    }

    /// Keeps `selected_address` pointing at a device that's actually
    /// visible under the current filter, picking the first visible
    /// device if the previous selection vanished or was never set. Call
    /// after anything that can change `devices` or `device_filter`.
    fn normalize_selection(&mut self) {
        if self.selected_visible_position().is_some() {
            return;
        }
        self.selected_address = self
            .visible_device_indices()
            .first()
            .map(|&i| self.devices[i].address());
    }

    /// Shows a success banner, replacing any currently showing.
    pub fn set_status_ok(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.status = Some(StatusBanner {
            kind: BannerKind::Info,
            title: title.into(),
            body: body.into(),
            shown_at: Instant::now(),
        });
    }

    /// Shows a failure banner (the error's `Display` as the body),
    /// replacing any currently showing.
    pub fn set_status_err(&mut self, title: impl Into<String>, err: &DriverError) {
        self.status = Some(StatusBanner {
            kind: BannerKind::Error,
            title: title.into(),
            body: err.to_string(),
            shown_at: Instant::now(),
        });
    }

    fn note_bluez_timeout(&mut self) {
        self.bluez_unresponsive_since.get_or_insert_with(Instant::now);
    }

    fn note_bluez_ok(&mut self) {
        self.bluez_unresponsive_since = None;
    }

    async fn refresh_devices(&mut self) {
        let Some(adapter) = self.adapters.get(self.adapter_idx) else {
            return;
        };
        match background_timeout(adapter.devices()).await {
            Ok(devices) => {
                self.devices = devices;
                self.devices_dirty = false;
                self.note_bluez_ok();
            }
            Err(DriverError::Timeout) => {
                // Leave devices_dirty set so the next tick (250ms) retries
                // automatically - bluetoothd not answering is usually
                // transient (a restart), and there's no other event to
                // wait for while it's down.
                self.note_bluez_timeout();
            }
            Err(e) => {
                self.devices_dirty = false;
                self.set_status_err("REFRESH FAILED", &e);
            }
        }
        // Keep the same device selected by address across the refresh -
        // RSSI (and therefore visible_device_indices()'s sort order)
        // changes constantly during an active scan.
        self.normalize_selection();
    }

    /// Periodic housekeeping: refreshes `devices` if dirty, expires the
    /// status banner, and refreshes battery/full-info if either is
    /// stale or the selection changed. Call this on a timer - both
    /// frontends use a 250ms one.
    pub async fn on_tick(&mut self) {
        if self.devices_dirty {
            self.refresh_devices().await;
        }
        if let Some(status) = &self.status
            && status.shown_at.elapsed() > STATUS_BANNER_LIFETIME
        {
            self.status = None;
        }
        self.refresh_battery_if_needed().await;
        self.refresh_full_info_if_needed().await;
    }

    async fn refresh_full_info_if_needed(&mut self) {
        if !self.fullscreen_detail {
            return;
        }
        let Some((address, connected)) = self
            .selected_device()
            .map(|d| (d.address(), d.is_connected()))
        else {
            self.full_info = None;
            self.full_info_for = None;
            return;
        };
        let selection_changed = self.full_info_for.as_ref() != Some(&address);
        let stale = self
            .full_info_checked_at
            .is_none_or(|t| t.elapsed() > FULL_INFO_REFRESH_INTERVAL);

        if !connected {
            if selection_changed {
                self.full_info = None;
                self.full_info_for = Some(address);
                self.full_info_checked_at = None;
            }
            return;
        }
        if !selection_changed && !stale {
            return;
        }

        self.full_info_for = Some(address);
        self.full_info_checked_at = Some(Instant::now());

        // Each `self.selected_device()` call's borrow ends with the
        // expression it's used in, so it never overlaps the `&mut self`
        // calls to note_bluez_*() below.
        let device_info_result = match self.selected_device() {
            Some(device) => background_timeout(device.device_information()).await,
            None => {
                self.full_info = None;
                return;
            }
        };

        match device_info_result {
            Ok(device_info) => {
                self.note_bluez_ok();
                let pnp_result = match self.selected_device() {
                    Some(device) => Some(background_timeout(device.pnp_id()).await),
                    None => None,
                };
                let pnp_id = match pnp_result {
                    Some(Ok(id)) => {
                        self.note_bluez_ok();
                        id
                    }
                    Some(Err(DriverError::Timeout)) => {
                        self.note_bluez_timeout();
                        None
                    }
                    Some(Err(_)) | None => None,
                };
                self.full_info = Some(FullDeviceInfo { device_info, pnp_id });
            }
            Err(DriverError::Timeout) => self.note_bluez_timeout(),
            Err(_) => {}
        }
    }

    async fn refresh_battery_if_needed(&mut self) {
        let Some((address, connected)) = self
            .selected_device()
            .map(|d| (d.address(), d.is_connected()))
        else {
            self.battery = None;
            self.battery_for = None;
            return;
        };
        let selection_changed = self.battery_for.as_ref() != Some(&address);
        let stale = self
            .battery_checked_at
            .is_none_or(|t| t.elapsed() > BATTERY_REFRESH_INTERVAL);

        if !connected {
            if selection_changed {
                self.battery = None;
                self.battery_for = Some(address);
                self.battery_checked_at = None;
            }
            return;
        }
        if !selection_changed && !stale {
            return;
        }

        self.battery_for = Some(address);
        self.battery_checked_at = Some(Instant::now());
        let result = match self.selected_device() {
            Some(device) => Some(background_timeout(device.battery_percent()).await),
            None => None,
        };
        self.battery = match result {
            Some(Ok(percent)) => {
                self.note_bluez_ok();
                percent
            }
            Some(Err(DriverError::Timeout)) => {
                self.note_bluez_timeout();
                None
            }
            Some(Err(_)) | None => None,
        };
    }

    /// Applies one live event from the driver's event stream: updates
    /// relevant state (last-seen timestamps, devices_dirty, ...) and
    /// appends it to the event log. Call this for every event the
    /// backend's `EventStream` yields.
    pub async fn handle_driver_event(&mut self, event: DriverEvent) {
        let touches_current_adapter = match self.current_adapter() {
            Some(adapter) => event_adapter_id(&event) == Some(adapter.id()),
            None => false,
        };
        if touches_current_adapter {
            match &event {
                DriverEvent::DeviceFound { address, .. } | DriverEvent::DeviceUpdated { address, .. } => {
                    self.last_seen.insert(address.clone(), Instant::now());
                    self.devices_dirty = true;
                }
                DriverEvent::DeviceRemoved { .. } => self.devices_dirty = true,
                DriverEvent::PoweredChanged { .. } | DriverEvent::DiscoveringChanged { .. } => {
                    let result = match self.adapters.get_mut(self.adapter_idx) {
                        Some(adapter) => Some(background_timeout(adapter.refresh()).await),
                        None => None,
                    };
                    match result {
                        Some(Ok(())) => self.note_bluez_ok(),
                        Some(Err(DriverError::Timeout)) => self.note_bluez_timeout(),
                        Some(Err(_)) | None => {}
                    }
                }
                DriverEvent::AdapterAdded(_) | DriverEvent::AdapterRemoved(_) => {}
            }
        }
        if matches!(self.overlay, Overlay::None) && !matches!(self.screen, Screen::EventLog) {
            self.unseen_events += 1;
        }
        self.events.push_back(LoggedEvent {
            at: Instant::now(),
            event,
        });
        while self.events.len() > MAX_LOGGED_EVENTS {
            self.events.pop_front();
        }
    }

    /// Applies one key press, dispatching to whatever overlay/screen
    /// is currently active. Call this for every key event the
    /// frontend's input source yields.
    pub async fn handle_key(&mut self, key: Key) {

        if self.overlay != Overlay::None {
            self.handle_overlay_key(key).await;
            return;
        }

        match key {
            Key::Char('q') => self.should_quit = true,
            Key::Char('?') => self.overlay = Overlay::Help,
            Key::Char(':') => {
                self.overlay = Overlay::Palette;
                self.palette_buffer.clear();
            }
            Key::Char('e') => {
                self.screen = Screen::EventLog;
                self.unseen_events = 0;
            }
            Key::Char('A') => self.screen = Screen::AdapterControl,
            Key::Char('s') => self.toggle_scan().await,
            Key::Esc => {
                if self.fullscreen_detail {
                    self.fullscreen_detail = false;
                } else {
                    self.screen = Screen::Shell;
                }
            }
            _ if self.screen == Screen::Shell => self.handle_shell_key(key).await,
            _ if self.screen == Screen::Discovery => self.handle_discovery_key(key).await,
            _ if self.screen == Screen::AdapterControl => self.handle_adapter_control_key(key).await,
            _ if self.screen == Screen::EventLog => self.handle_event_log_key(key),
            _ => {}
        }
    }

    async fn handle_shell_key(&mut self, key: Key) {
        match key {
            Key::Up => self.move_selection(-1),
            Key::Down => self.move_selection(1),
            Key::Right => self.drill_in(),
            Key::Left => self.drill_out(),
            Key::Tab => self.cycle_focus(),
            Key::Char('f') => self.fullscreen_detail = !self.fullscreen_detail,
            Key::Char('/') => {
                self.device_filter = self.device_filter.next();
                self.normalize_selection();
            }
            Key::Char('v') => self.open_vendor_modal().await,
            Key::Char('a') => self.begin_alias_edit(),
            Key::Char('r') => self.refresh_devices().await,
            Key::Enter => self.toggle_connection().await,
            Key::Char('p') => self.pair_selected().await,
            Key::Char('t') => self.set_trusted_selected(true).await,
            Key::Char('T') => self.set_trusted_selected(false).await,
            Key::Char('b') => self.set_blocked_selected(true).await,
            Key::Char('B') => self.set_blocked_selected(false).await,
            Key::Char('F') => self.overlay = Overlay::ConfirmForget,
            _ => {}
        }
    }

    async fn handle_discovery_key(&mut self, key: Key) {
        match key {
            Key::Up => self.move_selection(-1),
            Key::Down => self.move_selection(1),
            Key::Char('p') => self.pair_selected().await,
            Key::Char('c') => self.toggle_connection().await,
            Key::Char('v') => self.open_vendor_modal().await,
            _ => {}
        }
    }

    async fn handle_adapter_control_key(&mut self, key: Key) {
        let Some(adapter) = self.adapters.get_mut(self.adapter_idx) else {
            return;
        };
        match key {
            Key::Char('o') => {
                let target = !adapter.is_powered();
                if let Err(e) = adapter.set_powered(target).await {
                    self.set_status_err("POWER TOGGLE FAILED", &e);
                }
            }
            Key::Char('d') => {
                let target = !adapter.is_discoverable();
                if let Err(e) = adapter.set_discoverable(target).await {
                    self.set_status_err("SET DISCOVERABLE FAILED", &e);
                }
            }
            Key::Char('k') => {
                let target = !adapter.is_pairable();
                if let Err(e) = adapter.set_pairable(target).await {
                    self.set_status_err("SET PAIRABLE FAILED", &e);
                }
            }
            Key::Char('s') => self.toggle_scan().await,
            Key::Char('r') => {
                if let Err(e) = adapter.refresh().await {
                    self.set_status_err("REFRESH FAILED", &e);
                }
            }
            _ => {}
        }
    }

    fn handle_event_log_key(&mut self, key: Key) {
        // Scrolling is handled directly by the events screen renderer via
        // a persisted offset; nothing state-changing to do here yet
        // beyond what the top-level dispatcher already covers.
        let _ = key;
    }

    async fn handle_overlay_key(&mut self, key: Key) {
        match self.overlay {
            Overlay::AliasEdit => match key {
                Key::Enter => self.commit_alias_edit().await,
                Key::Esc => self.overlay = Overlay::None,
                Key::Backspace => {
                    self.alias_buffer.pop();
                }
                Key::Char(c) => self.alias_buffer.push(c),
                _ => {}
            },
            Overlay::Palette => match key {
                Key::Esc => self.overlay = Overlay::None,
                Key::Backspace => {
                    self.palette_buffer.pop();
                }
                Key::Char(c) => self.palette_buffer.push(c),
                Key::Enter => {
                    let cmd = PaletteCommand::filtered(&self.palette_buffer)
                        .first()
                        .map(|entry| entry.0);
                    match cmd {
                        Some(cmd) => self.run_palette_command(cmd).await,
                        None => self.overlay = Overlay::None,
                    }
                }
                _ => {}
            },
            Overlay::ConfirmForget => match key {
                Key::Char('y') | Key::Char('Y') => self.forget_selected().await,
                _ => self.overlay = Overlay::None,
            },
            Overlay::Vendor | Overlay::Help | Overlay::None => match key {
                Key::Esc | Key::Char('v') | Key::Char('?') => {
                    self.overlay = Overlay::None;
                }
                _ => {}
            },
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.focus {
            Focus::Adapters => {
                if !self.adapters.is_empty() {
                    let new_idx = wrap_index(self.adapter_idx, delta, self.adapters.len());
                    if new_idx != self.adapter_idx {
                        self.adapter_idx = new_idx;
                        // The device list belongs to the old adapter; drop
                        // it immediately rather than showing stale data
                        // from the wrong adapter until the next refresh.
                        self.devices.clear();
                        self.selected_address = None;
                        self.devices_dirty = true;
                    }
                }
            }
            Focus::Devices => {
                let visible = self.visible_device_indices();
                if visible.is_empty() {
                    return;
                }
                let current_pos = self.selected_visible_position().unwrap_or(0);
                let new_pos = wrap_index(current_pos, delta, visible.len());
                self.selected_address = Some(self.devices[visible[new_pos]].address());
            }
            Focus::Detail => {}
        }
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Adapters => Focus::Devices,
            Focus::Devices => Focus::Detail,
            Focus::Detail => Focus::Adapters,
        };
    }

    fn drill_in(&mut self) {
        self.focus = match self.focus {
            Focus::Adapters => Focus::Devices,
            Focus::Devices => Focus::Detail,
            Focus::Detail => Focus::Detail,
        };
    }

    fn drill_out(&mut self) {
        self.focus = match self.focus {
            Focus::Detail => Focus::Devices,
            Focus::Devices => Focus::Adapters,
            Focus::Adapters => Focus::Adapters,
        };
    }

    async fn toggle_scan(&mut self) {
        let Some(discovering) = self.adapters.get(self.adapter_idx).map(Adapter::is_discovering)
        else {
            return;
        };

        let result = if discovering {
            self.scan_started_at = None;
            let Some(adapter) = self.adapters.get_mut(self.adapter_idx) else {
                return;
            };
            adapter.stop_discovery().await
        } else {
            self.scan_started_at = Some(Instant::now());
            // Discovery is its own screen; reuse the same filtered-device
            // machinery the Shell screen already has, just pinned to
            // unpaired ("nearby") devices, rather than tracking a
            // parallel selection/list for it.
            self.screen = Screen::Discovery;
            self.device_filter = DeviceFilter::Nearby;
            self.normalize_selection();
            let Some(adapter) = self.adapters.get_mut(self.adapter_idx) else {
                return;
            };
            adapter.start_discovery().await
        };
        if let Err(e) = result {
            self.scan_started_at = None;
            self.set_status_err("SCAN TOGGLE FAILED", &e);
        }
    }

    async fn toggle_connection(&mut self) {
        let Some(device) = self.selected_device_mut() else {
            return;
        };
        let result = if device.is_connected() {
            device.disconnect().await
        } else {
            device.connect().await
        };
        if let Err(e) = result {
            self.set_status_err("CONNECT FAILED", &e);
        }
    }

    async fn pair_selected(&mut self) {
        let Some(device) = self.selected_device_mut() else {
            return;
        };
        if let Err(e) = device.pair().await {
            self.set_status_err("PAIR FAILED", &e);
        }
    }

    async fn set_trusted_selected(&mut self, trusted: bool) {
        let Some(device) = self.selected_device_mut() else {
            return;
        };
        if let Err(e) = device.set_trusted(trusted).await {
            self.set_status_err("TRUST CHANGE FAILED", &e);
        }
    }

    async fn set_blocked_selected(&mut self, blocked: bool) {
        let Some(device) = self.selected_device_mut() else {
            return;
        };
        if let Err(e) = device.set_blocked(blocked).await {
            self.set_status_err("BLOCK CHANGE FAILED", &e);
        }
    }

    async fn forget_selected(&mut self) {
        self.overlay = Overlay::None;
        let Some(device) = self.selected_device_mut() else {
            return;
        };
        let address = device.address();
        match device.remove().await {
            Ok(()) => {
                self.devices_dirty = true;
                self.set_status_ok("FORGOTTEN", format!("{address} removed from this adapter"));
            }
            Err(e) => self.set_status_err("FORGET FAILED", &e),
        }
    }

    fn begin_alias_edit(&mut self) {
        self.alias_buffer = self
            .selected_device()
            .and_then(Device::alias)
            .unwrap_or_default()
            .to_owned();
        self.overlay = Overlay::AliasEdit;
    }

    async fn commit_alias_edit(&mut self) {
        self.overlay = Overlay::None;
        let alias = self.alias_buffer.clone();
        let Some(device) = self.selected_device_mut() else {
            return;
        };
        match device.set_alias(&alias).await {
            Ok(()) => self.set_status_ok("ALIAS SET", format!("Alias = \"{alias}\"")),
            Err(e) => self.set_status_err("ALIAS FAILED", &e),
        }
    }

    async fn open_vendor_modal(&mut self) {
        let Some(device) = self.selected_device() else {
            return;
        };
        self.vendor_info = Some(VendorAttribution::compute(device).await);
        self.overlay = Overlay::Vendor;
    }

    async fn run_palette_command(&mut self, cmd: PaletteCommand) {
        self.overlay = Overlay::None;
        match cmd {
            PaletteCommand::Trust => self.set_trusted_selected(true).await,
            PaletteCommand::Untrust => self.set_trusted_selected(false).await,
            PaletteCommand::Block => self.set_blocked_selected(true).await,
            PaletteCommand::Unblock => self.set_blocked_selected(false).await,
            PaletteCommand::Pair => self.pair_selected().await,
            PaletteCommand::Connect => self.toggle_connection().await,
            // Forget always confirms first, same as the 'F' key.
            PaletteCommand::Forget => self.overlay = Overlay::ConfirmForget,
            PaletteCommand::Alias => self.begin_alias_edit(),
            PaletteCommand::ScanToggle => self.toggle_scan().await,
            PaletteCommand::Refresh => self.refresh_devices().await,
            PaletteCommand::Events => {
                self.screen = Screen::EventLog;
                self.unseen_events = 0;
            }
            PaletteCommand::AdapterControl => self.screen = Screen::AdapterControl,
            PaletteCommand::Help => self.overlay = Overlay::Help,
            PaletteCommand::Quit => self.should_quit = true,
        }
    }
}

/// A command palette entry: the command itself, its name (what `/`-style
/// substring matching runs against), a description, and the equivalent
/// direct keybinding if it has one.
pub struct PaletteEntry(pub PaletteCommand, pub &'static str, pub &'static str, pub Option<&'static str>);

/// One action reachable from the command palette - see `PaletteEntry::ALL`
/// for each command's name/description/keybinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    /// Mark the selected device trusted.
    Trust,
    /// Remove trust from the selected device.
    Untrust,
    /// Block the selected device.
    Block,
    /// Unblock the selected device.
    Unblock,
    /// Pair with the selected device.
    Pair,
    /// Connect/disconnect the selected device.
    Connect,
    /// Forget the selected device (removes bonding keys).
    Forget,
    /// Edit the selected device's alias.
    Alias,
    /// Start/stop discovery.
    ScanToggle,
    /// Refresh the device list.
    Refresh,
    /// Open the event log.
    Events,
    /// Open adapter control.
    AdapterControl,
    /// Show the keymap.
    Help,
    /// Quit.
    Quit,
}

impl PaletteCommand {
    const ALL: &'static [PaletteEntry] = &[
        PaletteEntry(Self::Trust, "trust", "Mark selected device trusted", Some("t")),
        PaletteEntry(Self::Untrust, "untrust", "Remove trust - auto-reconnect stops", Some("T")),
        PaletteEntry(Self::Block, "block", "Block the selected device", Some("b")),
        PaletteEntry(Self::Unblock, "unblock", "Unblock the selected device", Some("B")),
        PaletteEntry(Self::Pair, "pair", "Pair with the selected device", Some("p")),
        PaletteEntry(Self::Connect, "connect", "Connect/disconnect the selected device", Some("↵")),
        PaletteEntry(Self::Forget, "forget", "Forget the selected device (removes keys)", Some("F")),
        PaletteEntry(Self::Alias, "alias", "Edit the selected device's alias", Some("a")),
        PaletteEntry(Self::ScanToggle, "scan", "Start/stop discovery", Some("s")),
        PaletteEntry(Self::Refresh, "refresh", "Refresh the device list", Some("r")),
        PaletteEntry(Self::Events, "events", "Open the event log", Some("e")),
        PaletteEntry(Self::AdapterControl, "adapter", "Open adapter control", Some("A")),
        PaletteEntry(Self::Help, "help", "Show the keymap", Some("?")),
        PaletteEntry(Self::Quit, "quit", "Quit bluetooth-tui", Some("q")),
    ];

    /// Commands whose name contains `query`, case-insensitively. An empty
    /// query matches everything.
    pub fn filtered(query: &str) -> Vec<&'static PaletteEntry> {
        let query = query.to_lowercase();
        Self::ALL.iter().filter(|entry| entry.1.contains(&query)).collect()
    }
}

/// Races a background driver call against [`BACKGROUND_CALL_TIMEOUT`],
/// on top of (well inside) the driver's own connection-level timeout.
/// Dropping the losing future on timeout is safe: it's just an
/// in-progress reply read, discarded if one eventually arrives for a
/// call nobody's waiting on anymore.
///
/// Built on `futures-timer`/`futures-util::select` rather than
/// `tokio::time::timeout`: this crate is shared with a wasm32 frontend
/// that has no tokio runtime at all, and both of those crates work
/// unmodified on native and wasm32 alike.
async fn background_timeout<T>(
    fut: impl Future<Output = Result<T, DriverError>>,
) -> Result<T, DriverError> {
    let timer = futures_timer::Delay::new(BACKGROUND_CALL_TIMEOUT);
    futures_util::pin_mut!(fut);
    futures_util::pin_mut!(timer);
    match futures_util::future::select(fut, timer).await {
        futures_util::future::Either::Left((result, _)) => result,
        futures_util::future::Either::Right(_) => Err(DriverError::Timeout),
    }
}

fn wrap_index(current: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let len = len as i32;
    let next = (current as i32 + delta).rem_euclid(len);
    next as usize
}

fn event_adapter_id(event: &DriverEvent) -> Option<&bluetooth_driver::driver::AdapterId> {
    match event {
        DriverEvent::AdapterAdded(id) | DriverEvent::AdapterRemoved(id) => Some(id),
        DriverEvent::DeviceFound { adapter, .. }
        | DriverEvent::DeviceUpdated { adapter, .. }
        | DriverEvent::DeviceRemoved { adapter, .. }
        | DriverEvent::DiscoveringChanged { adapter, .. }
        | DriverEvent::PoweredChanged { adapter, .. } => Some(adapter),
    }
}
