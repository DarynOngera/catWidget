# 💜 Cat Love Notes Desktop Widget 💜

A romantic desktop widget built with Rust that displays adorable cat images, personalized love notes, and cat breed facts from The Cat API. Perfect as a thoughtful gift to brighten someone's day with cute cats and heartfelt messages.

## ✨ Features

- **Real Cat Images**: Fetches random cat photos from The Cat API
- **Romantic Love Notes**: 20 personalized cat-themed love messages
- **Cat Breed Facts**: Displays breed information when available
- **Purple Theme**: Beautiful purple color scheme (her favorite color!)
- **Heart Animation**: Floating hearts when clicking "New Cat!" button
- **Auto-Refresh**: Updates every hour automatically
- **Easter Egg**: Special message after 10 button clicks
- **Error Handling**: Graceful fallbacks when API is unavailable
- **Cross-Platform**: Works on Windows, Mac, and Linux

## 🚀 Quick Start

### Prerequisites
- Rust (1.70 or later)
- Internet connection for cat images

### Running the Application
```bash
# Clone or download the project
cd cat-love-notes

# Run in development mode
cargo run

# Build release version
cargo build --release
```

The release executable will be located at `target/release/cat-love-notes` (or `cat-love-notes.exe` on Windows).

## 🎨 Customization

### Adding Your Own Love Messages
Edit the `love_notes()` function in `src/main.rs` to add personalized messages:

```rust
fn love_notes() -> Vec<String> {
    vec![
        "You're pawsitively my favorite person! 🐾💜".to_string(),
        "My heart purrs for you every day! 😘".to_string(),
        // Add your own messages here!
        "Remember our first date at the cat cafe? 😻".to_string(),
    ]
}
```

### Using Your Own Cat API Key
For higher rate limits and more features, get a free API key from [The Cat API](https://thecatapi.com/) and modify the API call in `fetch_cat_data()`:

```rust
let response = client
    .get("https://api.thecatapi.com/v1/images/search")
    .header("x-api-key", "YOUR_API_KEY_HERE")
    .send()
    .await?;
```

### Changing Colors
Modify the purple theme in `setup_purple_theme()` function:

```rust
// Change these RGB values to your preferred colors
let purple_primary = egui::Color32::from_rgb(147, 112, 219);
let purple_light = egui::Color32::from_rgb(221, 160, 221);
let purple_dark = egui::Color32::from_rgb(75, 0, 130);
```

### Adjusting Auto-Refresh Timer
Change the refresh interval in the `new()` function:

```rust
auto_refresh_interval: Duration::from_secs(1800), // 30 minutes instead of 1 hour
```

## 🐱 How It Works

1. **Startup**: The app loads with a welcome message
2. **API Integration**: Fetches random cat images from `https://api.thecatapi.com/v1/images/search`
3. **Image Display**: Downloads and displays the cat image
4. **Love Notes**: Shows a random romantic message with cat puns
5. **Breed Info**: Displays cat breed facts when available
6. **Animations**: Heart animation plays when clicking the button
7. **Auto-Update**: Refreshes content every hour automatically

## 🛠️ Technical Details

- **Language**: Rust 2021 Edition
- **GUI Framework**: egui/eframe for cross-platform desktop UI
- **HTTP Client**: reqwest with rustls for secure API calls
- **Async Runtime**: tokio for non-blocking operations
- **Image Processing**: image crate for loading and displaying photos
- **Architecture**: Message-passing between async tasks and UI thread

## 📦 Dependencies

- `eframe` - Cross-platform GUI framework
- `egui` - Immediate mode GUI library
- `reqwest` - HTTP client for API calls
- `tokio` - Async runtime
- `serde` - JSON serialization/deserialization
- `rand` - Random number generation for message selection
- `image` - Image loading and processing

## 🎁 Gift Ideas

- **Personalization**: Add inside jokes and shared memories to love messages
- **Scheduling**: Set up as a startup program so it greets her every morning
- **Screenshots**: Take screenshots of cute cats and messages to share
- **Custom Icon**: Replace the default icon with a photo of you two

## 🐾 Love Messages Included

The widget comes with 20 romantic cat-themed messages:
- "You're pawsitively my favorite person! 🐾💜"
- "My heart purrs for you every day! 😘"
- "This cat's got nothing on your cuteness! 💖"
- "I'd share my catnip with you any day! 😻"
- "You make my heart do little cat zoomies! 🏃‍♀️💜"
- And 15 more adorable messages!

## 🎉 Easter Egg

Click the "New Cat!" button 10 times to unlock a special surprise message: 
*"🎉 SURPRISE! You've unlocked the secret message: You're my forever catnip and the love of my life! 💕🌿✨"*

## 🔧 Troubleshooting

**No images loading?**
- Check internet connection
- The Cat API might be temporarily down (fallback messages will show)

**App won't start?**
- Ensure you have the latest Rust version
- Try `cargo clean && cargo build --release`

**Want to contribute?**
- Add more love messages
- Implement sound effects
- Add system tray functionality
- Create custom themes

## 💝 Made with Love

This widget was created as a romantic gift to show someone special how much they mean to you. Every purr, every whisker, every adorable cat photo is a reminder of your love.

---

*"In a world full of dogs, you're my favorite cat person." 💜*
