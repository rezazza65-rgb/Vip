#!/bin/bash

# Advanced Proxy Scanner & Config Generator Setup Script
# This script sets up the entire project structure automatically

set -e

echo "🚀 Advanced Proxy Scanner & Config Generator Setup"
echo "=================================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust is not installed${NC}"
    echo "Please install Rust from: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✅ Rust is installed${NC}"

# Create necessary directories
echo ""
echo "📁 Creating directory structure..."
mkdir -p sub/configs/qr_codes
mkdir -p config_generator/src
mkdir -p .github/workflows

echo -e "${GREEN}✅ Directories created${NC}"

# Build scanner
echo ""
echo "🔨 Building proxy scanner..."
if cargo build --release; then
    echo -e "${GREEN}✅ Scanner built successfully${NC}"
else
    echo -e "${RED}❌ Scanner build failed${NC}"
    exit 1
fi

# Build config generator
echo ""
echo "🔨 Building config generator..."
cd config_generator
if cargo build --release; then
    echo -e "${GREEN}✅ Config generator built successfully${NC}"
else
    echo -e "${RED}❌ Config generator build failed${NC}"
    exit 1
fi
cd ..

# Set executable permissions
echo ""
echo "🔐 Setting executable permissions..."
chmod +x ./target/release/RScanner 2>/dev/null || chmod +x ./target/release/rscanner 2>/dev/null || true
chmod +x ./config_generator/target/release/config-generator 2>/dev/null || true

echo -e "${GREEN}✅ Permissions set${NC}"

# Create convenience scripts
echo ""
echo "📝 Creating convenience scripts..."

cat > run_scanner.sh << 'EOF'
#!/bin/bash
echo "🔍 Running Proxy Scanner..."
if [ -f "./target/release/RScanner" ]; then
    ./target/release/RScanner
elif [ -f "./target/release/rscanner" ]; then
    ./target/release/rscanner
else
    echo "Scanner binary not found. Please build first."
    exit 1
fi
EOF

cat > run_generator.sh << 'EOF'
#!/bin/bash
echo "⚙️  Running Config Generator..."
if [ -f "./config_generator/target/release/config-generator" ]; then
    ./config_generator/target/release/config-generator
else
    echo "Config generator binary not found. Please build first."
    exit 1
fi
EOF

cat > run_full_pipeline.sh << 'EOF'
#!/bin/bash
echo "🚀 Running Full Pipeline"
echo "======================="
echo ""

# Run scanner
echo "Phase 1: Scanning Proxies"
echo "-------------------------"
./run_scanner.sh
echo ""

# Check if scanner created output
if [ ! -f "sub/ProxyIP-Daily.md" ]; then
    echo "⚠️  Scanner output not found. Skipping config generation."
    exit 1
fi

# Run config generator
echo "Phase 2: Generating Configurations"
echo "----------------------------------"
./run_generator.sh
echo ""

# Display results
echo "✅ Pipeline Complete!"
echo ""
echo "📊 Results:"
echo "  - Proxy list: sub/ProxyIP-Daily.md"
echo "  - Configs: sub/configs/"
echo "  - Subscription: sub/configs/subscription.txt"
echo "  - Reports: sub/configs/README.md"
echo "  - Statistics: sub/configs/statistics.txt"
echo ""
EOF

chmod +x run_scanner.sh run_generator.sh run_full_pipeline.sh

echo -e "${GREEN}✅ Convenience scripts created${NC}"

# Test installation
echo ""
echo "🧪 Testing installation..."

if [ -f "./target/release/RScanner" ] || [ -f "./target/release/rscanner" ]; then
    echo -e "${GREEN}✅ Scanner binary found${NC}"
else
    echo -e "${YELLOW}⚠️  Scanner binary not found at expected location${NC}"
fi

if [ -f "./config_generator/target/release/config-generator" ]; then
    echo -e "${GREEN}✅ Config generator binary found${NC}"
else
    echo -e "${YELLOW}⚠️  Config generator binary not found at expected location${NC}"
fi

# Summary
echo ""
echo "=================================================="
echo -e "${GREEN}🎉 Setup Complete!${NC}"
echo "=================================================="
echo ""
echo "Available Commands:"
echo "  ./run_scanner.sh          - Run proxy scanner only"
echo "  ./run_generator.sh        - Run config generator only"
echo "  ./run_full_pipeline.sh    - Run complete pipeline"
echo ""
echo "Manual Execution:"
echo "  Scanner:    ./target/release/RScanner"
echo "  Generator:  ./config_generator/target/release/config-generator"
echo ""
echo "Output Locations:"
echo "  Proxy List:       sub/ProxyIP-Daily.md"
echo "  Configurations:   sub/configs/"
echo "  Subscription:     sub/configs/subscription.txt"
echo "  JSON Data:        sub/configs/configs.json"
echo "  Statistics:       sub/configs/statistics.txt"
echo "  QR Codes:         sub/configs/qr_codes/"
echo ""
echo "GitHub Actions:"
echo "  The workflow will run automatically every 24 hours"
echo "  Manual trigger: GitHub Actions > Proxy Scanner & Config Generator > Run workflow"
echo ""
echo "📚 Documentation: See PROJECT_README.md for detailed information"
echo ""
