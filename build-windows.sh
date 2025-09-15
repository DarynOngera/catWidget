#!/bin/bash

# Build script for creating Windows executable from Linux
echo "🚀 Building Cat Widget for Windows..."

# Install Windows target if not already installed
rustup target add x86_64-pc-windows-gnu

# Update dependencies to fix yanked versions
echo "🔄 Updating dependencies..."
cargo update

# Clean previous builds to avoid cache issues
echo "🧹 Cleaning previous builds..."
cargo clean

# Build for Windows with explicit linker
echo "📦 Cross-compiling for Windows..."
PKG_CONFIG_ALLOW_CROSS=1 \
RUSTFLAGS="-C linker=x86_64-w64-mingw32-gcc -C target-feature=+crt-static" \
cargo build --release --target x86_64-pc-windows-gnu

# Create distribution directory
mkdir -p dist/windows

# Check if build was successful
if [ -f "target/x86_64-pc-windows-gnu/release/cat-love-notes.exe" ]; then
    cp target/x86_64-pc-windows-gnu/release/cat-love-notes.exe dist/windows/CatWidget.exe
    
    # Create user-friendly README
    cat > dist/windows/README.txt << 'EOF'
🐱💜 Cat Widget - Desktop Love Notes

INSTALLATION:
1. Simply double-click "CatWidget.exe" to run
2. No installation required - it's portable!
3. The app will show adorable cats with love messages

FEATURES:
- Click "Show Me Love" for new cats and messages
- Hover over cat images to favorite them with the heart button
- Click "My Favorites" to see your saved cats
- Easter egg: Click 10 times for a special surprise!

REQUIREMENTS:
- Windows 7 or newer
- Internet connection for cat images

TROUBLESHOOTING:
- If Windows Defender blocks it, click "More info" then "Run anyway"
- The app is safe - it only fetches cute cat pictures!

Made with ❤️ for someone special
EOF

    echo "✅ Windows build complete!"
    echo "📁 Files created in: dist/windows/"
    echo "📋 Ready to share: CatWidget.exe + README.txt"
    echo "📦 File size: $(du -h dist/windows/CatWidget.exe | cut -f1)"
else
    echo "❌ Build failed - executable not found"
    echo "🔍 Check the build output above for errors"
    echo "💡 Try running: cargo build --target x86_64-pc-windows-gnu --verbose"
fi
