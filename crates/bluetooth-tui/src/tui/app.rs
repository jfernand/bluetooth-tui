use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use bluetooth_driver::bluez::{BluezAdapter, BluezDevice, BluezDriver};
use bluetooth_driver::driver::{
    Adapter, Address, BluetoothDriver, Device, DeviceInfo, DriverError, DriverEvent, PnpId,
    VendorIdSource,
};

const BATTERY_REFRESH_INTERVAL: Duration = Duration::from_secs(10);
const FULL_INFO_REFRESH_INTERVAL: Duration = Duration::from_secs(15);

const MAX_LOGGED_EVENTS: usize = 5_000;
const STATUS_BANNER_LIFETIME: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Shell,
    Discovery,
    EventLog,
    AdapterControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Vendor,
    AliasEdit,
    Help,
    Palette,
    ConfirmForget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Adapters,
    Devices,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFilter {
    All,
    Paired,
    Connected,
    Nearby,
    Blocked,
}

impl DeviceFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Paired => "PAIRED",
            Self::Connected => "CONNECTED",
            Self::Nearby => "NEARBY",
            Self::Blocked => "BLOCKED",
        }
    }

    pub fn matches(self, device: &BluezDevice) -> bool {
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

pub enum BannerKind {
    Info,
    Error,
}

pub struct StatusBanner {
    pub kind: BannerKind,
    pub title: String,
    pub body: String,
    pub shown_at: Instant,
}

pub struct LoggedEvent {
    pub at: Instant,
    pub event: DriverEvent,
}

/// One tier of the vendor-attribution chain, as shown in the vendor
/// modal - each tier states plainly what it actually proves.
pub struct VendorTier {
    pub label: &'static str,
    pub caveat: &'static str,
    pub result: Option<String>,
    pub confidence: u8, // 0-3 dots
}

pub struct VendorAttribution {
    pub resolved: Option<String>,
    pub tiers: Vec<VendorTier>,
}

/// A quick, synchronous-only vendor guess for list rows (OUI + beacon
/// company ID) - the full chain, including a live GATT PnP ID read, is
/// only worth the round trip when the user actually opens the vendor
/// modal for one specific device.
pub fn quick_vendor_label(device: &BluezDevice) -> Option<String> {
    let address = device.address();
    if let Some(vendor) = bluetooth_driver::oui::vendor(address) {
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
    pub async fn compute(device: &BluezDevice) -> Self {
        let address = device.address();

        let oui = bluetooth_driver::oui::vendor(address).map(str::to_owned);
        let oui_caveat = if oui.is_some() {
            "Identifies who registered the radio chip's address block."
        } else if bluetooth_driver::oui::is_locally_administered(address) {
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

pub struct App {
    pub driver: BluezDriver,
    pub adapters: Vec<BluezAdapter>,
    pub adapter_idx: usize,
    pub devices: Vec<BluezDevice>,
    pub device_filter: DeviceFilter,
    pub device_idx: usize,
    pub focus: Focus,
    pub fullscreen_detail: bool,
    pub screen: Screen,
    pub overlay: Overlay,
    pub alias_buffer: String,
    pub palette_buffer: String,
    pub events: VecDeque<LoggedEvent>,
    pub unseen_events: usize,
    pub status: Option<StatusBanner>,
    pub scan_started_at: Option<Instant>,
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
    pub devices_dirty: bool,
    pub should_quit: bool,
}

pub struct FullDeviceInfo {
    pub device_info: DeviceInfo,
    pub pnp_id: Option<PnpId>,
}

impl App {
    pub async fn new(driver: BluezDriver) -> Result<Self, DriverError> {
        let adapters = driver.adapters().await?;
        let devices = if let Some(adapter) = adapters.first() {
            adapter.devices().await?
        } else {
            Vec::new()
        };
        // If discovery was already running before this app started (e.g.
        // left on from bluetoothctl), we don't know the true start time -
        // count from now rather than showing a permanently-stuck 00:00.
        let scan_started_at = adapters
            .first()
            .filter(|a| a.is_discovering())
            .map(|_| Instant::now());

        Ok(Self {
            driver,
            adapters,
            adapter_idx: 0,
            devices,
            device_filter: DeviceFilter::All,
            device_idx: 0,
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
            devices_dirty: false,
            should_quit: false,
        })
    }

    pub fn current_adapter(&self) -> Option<&BluezAdapter> {
        self.adapters.get(self.adapter_idx)
    }

    pub fn visible_device_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.device_filter.matches(d))
            .map(|(i, _)| i)
            .collect();
        indices.sort_by_key(|&i| {
            std::cmp::Reverse(self.devices[i].rssi().map(|r| r.0).unwrap_or(i16::MIN))
        });
        indices
    }

    pub fn selected_device(&self) -> Option<&BluezDevice> {
        let visible = self.visible_device_indices();
        visible
            .get(self.device_idx)
            .and_then(|&i| self.devices.get(i))
    }

    fn selected_device_mut(&mut self) -> Option<&mut BluezDevice> {
        let visible = self.visible_device_indices();
        let real_idx = *visible.get(self.device_idx)?;
        self.devices.get_mut(real_idx)
    }

    pub fn set_status_ok(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.status = Some(StatusBanner {
            kind: BannerKind::Info,
            title: title.into(),
            body: body.into(),
            shown_at: Instant::now(),
        });
    }

    pub fn set_status_err(&mut self, title: impl Into<String>, err: &DriverError) {
        self.status = Some(StatusBanner {
            kind: BannerKind::Error,
            title: title.into(),
            body: err.to_string(),
            shown_at: Instant::now(),
        });
    }

    async fn refresh_devices(&mut self) {
        let Some(adapter) = self.adapters.get(self.adapter_idx) else {
            return;
        };
        match adapter.devices().await {
            Ok(devices) => self.devices = devices,
            Err(e) => self.set_status_err("REFRESH FAILED", &e),
        }
        self.devices_dirty = false;
    }

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
        let selection_changed = self.full_info_for != Some(address);
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
        self.full_info = match self.selected_device() {
            Some(device) => {
                let device_info = device.device_information().await.unwrap_or_default();
                let pnp_id = device.pnp_id().await.ok().flatten();
                Some(FullDeviceInfo { device_info, pnp_id })
            }
            None => None,
        };
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
        let selection_changed = self.battery_for != Some(address);
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
        self.battery = match self.selected_device() {
            Some(device) => device.battery_percent().await.ok().flatten(),
            None => None,
        };
    }

    pub async fn handle_driver_event(&mut self, event: DriverEvent) {
        let touches_current_adapter = match self.current_adapter() {
            Some(adapter) => event_adapter_id(&event) == Some(adapter.id()),
            None => false,
        };
        if touches_current_adapter {
            match &event {
                DriverEvent::DeviceFound { address, .. } | DriverEvent::DeviceUpdated { address, .. } => {
                    self.last_seen.insert(*address, Instant::now());
                    self.devices_dirty = true;
                }
                DriverEvent::DeviceRemoved { .. } => self.devices_dirty = true,
                DriverEvent::PoweredChanged { .. } | DriverEvent::DiscoveringChanged { .. } => {
                    if let Some(adapter) = self.adapters.get_mut(self.adapter_idx) {
                        let _ = adapter.refresh().await;
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

    pub async fn handle_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        if self.overlay != Overlay::None {
            self.handle_overlay_key(key).await;
            return;
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.overlay = Overlay::Help,
            KeyCode::Char(':') => {
                self.overlay = Overlay::Palette;
                self.palette_buffer.clear();
            }
            KeyCode::Char('e') => {
                self.screen = Screen::EventLog;
                self.unseen_events = 0;
            }
            KeyCode::Char('A') => self.screen = Screen::AdapterControl,
            KeyCode::Char('s') => self.toggle_scan().await,
            KeyCode::Esc => {
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

    async fn handle_shell_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match key {
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Right => self.drill_in(),
            KeyCode::Left => self.drill_out(),
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::Char('f') => self.fullscreen_detail = !self.fullscreen_detail,
            KeyCode::Char('/') => self.device_filter = self.device_filter.next(),
            KeyCode::Char('v') => self.open_vendor_modal().await,
            KeyCode::Char('a') => self.begin_alias_edit(),
            KeyCode::Char('r') => self.refresh_devices().await,
            KeyCode::Enter => self.toggle_connection().await,
            KeyCode::Char('p') => self.pair_selected().await,
            KeyCode::Char('t') => self.set_trusted_selected(true).await,
            KeyCode::Char('T') => self.set_trusted_selected(false).await,
            KeyCode::Char('b') => self.set_blocked_selected(true).await,
            KeyCode::Char('B') => self.set_blocked_selected(false).await,
            KeyCode::Char('F') => self.overlay = Overlay::ConfirmForget,
            _ => {}
        }
    }

    async fn handle_discovery_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match key {
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Char('p') => self.pair_selected().await,
            KeyCode::Char('c') => self.toggle_connection().await,
            KeyCode::Char('v') => self.open_vendor_modal().await,
            _ => {}
        }
    }

    async fn handle_adapter_control_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        let Some(adapter) = self.adapters.get_mut(self.adapter_idx) else {
            return;
        };
        match key {
            KeyCode::Char('o') => {
                let target = !adapter.is_powered();
                if let Err(e) = adapter.set_powered(target).await {
                    self.set_status_err("POWER TOGGLE FAILED", &e);
                }
            }
            KeyCode::Char('d') => {
                let target = !adapter.is_discoverable();
                if let Err(e) = adapter.set_discoverable(target).await {
                    self.set_status_err("SET DISCOVERABLE FAILED", &e);
                }
            }
            KeyCode::Char('k') => {
                let target = !adapter.is_pairable();
                if let Err(e) = adapter.set_pairable(target).await {
                    self.set_status_err("SET PAIRABLE FAILED", &e);
                }
            }
            KeyCode::Char('s') => self.toggle_scan().await,
            KeyCode::Char('r') => {
                if let Err(e) = adapter.refresh().await {
                    self.set_status_err("REFRESH FAILED", &e);
                }
            }
            _ => {}
        }
    }

    fn handle_event_log_key(&mut self, key: crossterm::event::KeyCode) {
        // Scrolling is handled directly by the events screen renderer via
        // a persisted offset; nothing state-changing to do here yet
        // beyond what the top-level dispatcher already covers.
        let _ = key;
    }

    async fn handle_overlay_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match self.overlay {
            Overlay::AliasEdit => match key {
                KeyCode::Enter => self.commit_alias_edit().await,
                KeyCode::Esc => self.overlay = Overlay::None,
                KeyCode::Backspace => {
                    self.alias_buffer.pop();
                }
                KeyCode::Char(c) => self.alias_buffer.push(c),
                _ => {}
            },
            Overlay::Palette => match key {
                KeyCode::Esc => self.overlay = Overlay::None,
                KeyCode::Backspace => {
                    self.palette_buffer.pop();
                }
                KeyCode::Char(c) => self.palette_buffer.push(c),
                KeyCode::Enter => {
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
                KeyCode::Char('y') | KeyCode::Char('Y') => self.forget_selected().await,
                _ => self.overlay = Overlay::None,
            },
            Overlay::Vendor | Overlay::Help | Overlay::None => match key {
                KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('?') => {
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
                        self.device_idx = 0;
                        self.devices_dirty = true;
                    }
                }
            }
            Focus::Devices => {
                let count = self.visible_device_indices().len();
                if count > 0 {
                    self.device_idx = wrap_index(self.device_idx, delta, count);
                }
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
        let Some(adapter) = self.adapters.get_mut(self.adapter_idx) else {
            return;
        };
        let result = if adapter.is_discovering() {
            self.scan_started_at = None;
            adapter.stop_discovery().await
        } else {
            self.scan_started_at = Some(Instant::now());
            // Discovery is its own screen; reuse the same filtered-device
            // machinery the Shell screen already has, just pinned to
            // unpaired ("nearby") devices, rather than tracking a
            // parallel selection/list for it.
            self.screen = Screen::Discovery;
            self.device_filter = DeviceFilter::Nearby;
            self.device_idx = 0;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    Trust,
    Untrust,
    Block,
    Unblock,
    Pair,
    Connect,
    Forget,
    Alias,
    ScanToggle,
    Refresh,
    Events,
    AdapterControl,
    Help,
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
