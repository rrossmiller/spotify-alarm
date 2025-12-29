# Spotify Alarm CLI

A command-line alarm clock that plays Spotify music at scheduled times with day-of-week filtering.

## Current Implementation

### Features

#### ✅ Alarm Scheduling
- **JSON Configuration**: Load alarms from a `alarms.json` configuration file
- **Multiple Alarms**: Define multiple alarms with different schedules
- **Time-based Triggering**: Alarms trigger at specific times in 24-hour format (HH:MM)
- **Day-of-Week Filtering**: Configure alarms to only play on specific days (Mon-Sun)
- **Weekday-only Alarms**: Perfect for work/school schedules (Monday-Friday)
- **Weekend Alarms**: Separate alarm times for Saturday and Sunday
- **Enable/Disable**: Individual alarms can be toggled without removing them

#### ✅ Spotify Integration
- **OAuth Authentication**: Automatic browser-based login on first run
- **Credential Caching**: Stores credentials in `.cache` directory
- **Playlist Playback**: Plays random tracks from a configured Spotify playlist
- **Maximum Volume**: Sets volume to maximum when alarm triggers

#### ✅ Command-Line Interface
- **Default Config**: Automatically looks for `alarms.json` in current directory
- **Custom Config Path**: Accepts config file path as command-line argument
- **Helpful Errors**: Shows example configuration if file is missing
- **Status Display**: Lists all configured alarms on startup

### Architecture

```
src/
├── main.rs      - Entry point, config loading, scheduler initialization
├── alarm.rs     - Alarm configuration, parsing, and scheduling logic
└── spotify.rs   - Spotify authentication and playback via librespot
```

### Configuration Format

```json
{
  "alarms": [
    {
      "name": "Weekday Morning Alarm",
      "time": "07:00",
      "days": ["Mon", "Tue", "Wed", "Thu", "Fri"],
      "enabled": true
    },
    {
      "name": "Weekend Alarm",
      "time": "09:00",
      "days": ["Sat", "Sun"],
      "enabled": true
    },
    {
      "name": "Daily Afternoon Reminder",
      "time": "15:30",
      "days": [],
      "enabled": false
    }
  ]
}
```

**Configuration Fields:**
- `name`: Human-readable alarm description
- `time`: 24-hour format time string (HH:MM)
- `days`: Array of days (Mon, Tue, Wed, Thu, Fri, Sat, Sun). Empty array = every day
- `enabled`: Boolean flag to activate/deactivate alarm

## Installation

### Raspberry Pi Setup

Install required libraries:
```bash
sudo apt-get install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0
```

### General Setup

1. Clone the repository
2. Install Rust (if not already installed): https://rustup.rs/
3. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

```bash
# Create your alarm configuration
cp alarms.json.example alarms.json
# Edit alarms.json with your desired alarm times

# Run with default config (alarms.json)
cargo run --release

# Run with custom config file
cargo run --release -- /path/to/my-alarms.json

# Or run the built binary directly
./target/release/spotify-alarm-cli
```

### First Run

On first run, the program will:
1. Open your browser for Spotify OAuth authentication
2. Ask you to log in and grant permissions
3. Cache your credentials in `.cache/` directory
4. Subsequent runs will use cached credentials

## Implemented But Unused

The following functions are implemented but currently not utilized:

### 1. `AlarmConfig::save()` (alarm.rs:73)
```rust
pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
```
**Purpose**: Save alarm configuration back to JSON file
**Potential Use**: Could be used for runtime alarm modifications or a future management interface

### 2. `seconds_until_time()` (alarm.rs:81)
```rust
fn seconds_until_time(target_time: NaiveTime) -> u64
```
**Purpose**: Calculate seconds until next occurrence of a specific time
**Potential Use**: Could optimize sleep intervals instead of checking every 10 seconds, or display "time until next alarm"

### 3. Single-Play Limitation
The current implementation consumes the Spotify connection after the first alarm plays, causing the program to exit. The scheduler is designed to check multiple alarms but can only trigger one before terminating.

## Future Enhancements

### High Priority

#### 🔄 Persistent Alarm Monitoring
**Current Issue**: Program exits after first alarm plays
**Solution**: Refactor Spotify connection to be reusable without consuming `spirc` and `spirc_task`
**Benefit**: True continuous alarm scheduling - multiple alarms per day

#### 🎵 Per-Alarm Playlist Configuration
```json
{
  "name": "Morning Workout",
  "time": "06:00",
  "playlist_uri": "spotify:playlist:37i9dQZF1DX76Wlfdnj7AP",
  "days": ["Mon", "Wed", "Fri"]
}
```
**Benefit**: Different music for different times/moods

#### 🔊 Volume Control
```json
{
  "name": "Gentle Wake-up",
  "time": "07:00",
  "volume": 50,
  "fade_in_seconds": 30
}
```
**Features**:
- Per-alarm volume levels (0-100)
- Gradual volume fade-in for gentle waking
- Configurable fade duration

### Medium Priority

#### ⏰ Snooze Functionality
- Add a snooze duration (e.g., 5-10 minutes)
- Keyboard interrupt handling to trigger snooze
- Configurable snooze count limit

#### 📊 Smart Scheduling
```rust
// Calculate next alarm time
fn next_alarm_time(&self) -> Option<DateTime<Local>>

// Sleep until next alarm instead of polling
async fn sleep_until_next_alarm()
```
**Benefits**:
- More efficient CPU usage
- Display "Next alarm at: X:XX AM" on startup
- Better battery life on laptops

#### 📝 Alarm History & Logging
- Log when alarms trigger
- Track snooze history
- Missed alarm detection
- Statistics (average wake time, snooze frequency)

#### 🔔 Multiple Alarm Actions
```json
{
  "actions": [
    {"type": "spotify", "playlist": "..."},
    {"type": "notification", "message": "Time to wake up!"},
    {"type": "command", "exec": "~/scripts/morning-routine.sh"}
  ]
}
```

### Low Priority / Nice to Have

#### 🌐 Web Interface
- REST API for alarm management
- Web UI for creating/editing alarms
- Mobile-responsive design
- Real-time alarm status

#### 🔄 Dynamic Configuration Reloading
- Watch `alarms.json` for changes
- Reload configuration without restarting
- Validate changes before applying

#### 🧪 Testing & Validation
- Unit tests for alarm parsing
- Integration tests for scheduling logic
- Config validation with helpful error messages
- Mock time for testing specific scenarios

#### 📱 System Integration
- macOS: Native notifications, menu bar app
- Linux: systemd service, desktop notifications
- Windows: Task scheduler integration, system tray

#### 🎨 Advanced Features
- **Sunrise simulation**: Gradually increase brightness of smart lights
- **Weather-based delays**: Adjust alarm time based on weather/commute
- **Sleep tracking integration**: Optimal wake time during light sleep
- **Multi-device sync**: Share alarm configs across computers
- **Voice commands**: Alexa/Google Home integration

## Known Limitations

1. **Single Alarm Playback**: Program exits after first alarm due to Spotify connection being consumed
2. **No Snooze**: Once an alarm plays, it cannot be snoozed
3. **Fixed Playlist**: Hardcoded playlist URI in `spotify.rs:75`
4. **No Volume Control**: Always plays at maximum volume
5. **Polling-based**: Checks time every 10 seconds instead of sleeping until next alarm
6. **No Error Recovery**: If Spotify playback fails, alarm is lost
7. **No Notification**: Silent failure if music doesn't play (e.g., no internet)

## Dependencies

- `librespot` - Spotify client library
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `serde` / `serde_json` - Configuration serialization
- `rand` - Random track selection
- `env_logger` / `log` - Logging

## Contributing

When implementing new features, consider:
- Maintaining backward compatibility with existing configs
- Adding sensible defaults for new fields
- Updating `alarms.json.example` with new options
- Error handling for network/Spotify issues

## Troubleshooting

### Authentication Issues
- Delete `.cache/` directory and re-authenticate
- Ensure you have an active Spotify Premium account (required for playback)

### No Sound
- Check system audio output settings
- Verify Spotify credentials are valid
- Check internet connection

### Alarm Doesn't Trigger
- Verify system time is correct
- Check alarm is `"enabled": true` in config
- Ensure current day matches alarm's `days` array
