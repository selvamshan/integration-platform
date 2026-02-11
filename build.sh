#!/bin/bash

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║   Integration Platform - Smart Build Script           ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Function to build with Docker
build_with_docker() {
    echo -e "${BLUE}🐳 Building with Docker...${NC}"
    echo -e "${YELLOW}This may take 10-20 minutes on first build...${NC}"
    echo ""
    
    # Try building with BuildKit
    if DOCKER_BUILDKIT=1 docker-compose build 2>&1 | tee /tmp/docker-build.log; then
        echo ""
        echo -e "${GREEN}✅ Docker build successful!${NC}"
        return 0
    else
        echo ""
        echo -e "${RED}❌ Docker build failed${NC}"
        
        # Check for common issues
        if grep -q "exit code: 101" /tmp/docker-build.log; then
            echo ""
            echo -e "${YELLOW}This looks like a memory issue.${NC}"
            echo ""
            echo "Solutions:"
            echo "  1. Increase Docker memory to 8GB (Settings → Resources → Memory)"
            echo "  2. Try local build: cargo build --release (if Rust installed)"
            echo "  3. Use debug build (edit Dockerfiles, remove --release)"
            echo ""
            echo "See TROUBLESHOOTING.md for detailed help."
        fi
        
        return 1
    fi
}

# Function to build locally
build_locally() {
    echo -e "${BLUE}🦀 Building locally with Rust...${NC}"
    
    # Check for PostgreSQL libraries
    if ! pkg-config --exists libpq 2>/dev/null; then
        echo -e "${RED}❌ PostgreSQL development libraries not found${NC}"
        echo ""
        echo "Install with:"
        echo "  Ubuntu/Debian: sudo apt-get install libpq-dev"
        echo "  macOS: brew install postgresql"
        echo ""
        return 1
    fi
    
    if cargo build --release; then
        echo ""
        echo -e "${GREEN}✅ Local build successful!${NC}"
        return 0
    else
        echo ""
        echo -e "${RED}❌ Local build failed${NC}"
        return 1
    fi
}

# Start PostgreSQL
start_postgres() {
    echo -e "${BLUE}🐘 Starting PostgreSQL...${NC}"
    docker-compose up -d postgres
    
    echo -e "${YELLOW}Waiting for PostgreSQL to be ready...${NC}"
    for i in {1..30}; do
        if docker-compose exec -T postgres pg_isready -U platform >/dev/null 2>&1; then
            echo -e "${GREEN}✅ PostgreSQL is ready${NC}"
            return 0
        fi
        sleep 1
        echo -n "."
    done
    
    echo ""
    echo -e "${RED}❌ PostgreSQL failed to start${NC}"
    docker-compose logs postgres
    return 1
}

# Main script
main() {
    # Check if Docker is running
    if ! docker info > /dev/null 2>&1; then
        echo -e "${RED}❌ Docker is not running. Please start Docker and try again.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Docker is running${NC}"
    echo ""
    
    # Show menu
    echo "Choose build method:"
    echo "  1) Docker build (no Rust installation needed)"
    echo "  2) Local Rust build (faster, requires Rust + libpq-dev)"
    echo "  3) Quick start with pre-check"
    echo ""
    read -p "Selection [3]: " BUILD_METHOD
    BUILD_METHOD=${BUILD_METHOD:-3}
    
    echo ""
    
    if [ "$BUILD_METHOD" = "3" ]; then
        # Auto-detect
        if command -v cargo &> /dev/null && pkg-config --exists libpq 2>/dev/null; then
            echo -e "${BLUE}✅ Rust and PostgreSQL libraries detected${NC}"
            echo -e "${BLUE}Building locally (faster)...${NC}"
            echo ""
            
            if build_locally; then
                # Start PostgreSQL
                start_postgres || exit 1
                
                # Set environment
                export DATABASE_URL="postgresql://platform:platform123@localhost:5432/integration_platform"
                
                # Start services
                echo ""
                echo -e "${GREEN}Starting services...${NC}"
                RUST_LOG=control_plane=debug,info ./target/release/control-plane > control-plane.log 2>&1 &
                CONTROL_PID=$!
                
                sleep 3
                
                RUST_LOG=data_plane=debug,integration_runtime=debug,info ./target/release/data-plane > data-plane.log 2>&1 &
                DATA_PID=$!
                
                echo "$CONTROL_PID $DATA_PID" > .service-pids
                
                echo -e "${GREEN}✅ Services started!${NC}"
                echo ""
                echo "  - Control Plane: http://localhost:8081 (PID: $CONTROL_PID)"
                echo "  - Data Plane:    http://localhost:8080 (PID: $DATA_PID)"
                echo ""
                echo "Logs:"
                echo "  - control-plane.log"
                echo "  - data-plane.log"
                echo ""
                echo "To stop: kill $CONTROL_PID $DATA_PID && docker-compose down"
            else
                echo -e "${YELLOW}Local build failed, trying Docker...${NC}"
                BUILD_METHOD=1
            fi
        else
            echo -e "${BLUE}Rust not fully configured, using Docker...${NC}"
            BUILD_METHOD=1
        fi
    fi
    
    if [ "$BUILD_METHOD" = "1" ]; then
        if build_with_docker; then
            echo ""
            echo -e "${BLUE}Starting services...${NC}"
            docker-compose up -d
            
            echo ""
            echo -e "${YELLOW}Waiting for services to start...${NC}"
            sleep 15
            
            echo -e "${GREEN}✅ Services started!${NC}"
            echo ""
            echo "  - Control Plane: http://localhost:8081"
            echo "  - Data Plane:    http://localhost:8080"
            echo ""
        else
            exit 1
        fi
    elif [ "$BUILD_METHOD" = "2" ]; then
        if build_locally; then
            start_postgres || exit 1
            
            export DATABASE_URL="postgresql://platform:platform123@localhost:5432/integration_platform"
            
            echo ""
            echo -e "${GREEN}Starting services...${NC}"
            RUST_LOG=control_plane=debug,info ./target/release/control-plane &
            CONTROL_PID=$!
            
            sleep 3
            
            RUST_LOG=data_plane=debug,integration_runtime=debug,info ./target/release/data-plane &
            DATA_PID=$!
            
            echo "$CONTROL_PID $DATA_PID" > .service-pids
            
            echo -e "${GREEN}✅ Services started!${NC}"
            echo ""
            echo "  - Control Plane: http://localhost:8081 (PID: $CONTROL_PID)"
            echo "  - Data Plane:    http://localhost:8080 (PID: $DATA_PID)"
            echo ""
        else
            exit 1
        fi
    fi
    
    # Test services
    echo -e "${BLUE}Testing services...${NC}"
    sleep 5
    
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Data Plane is healthy${NC}"
    else
        echo -e "${YELLOW}⚠️  Data Plane not responding (may need more time)${NC}"
    fi
    
    if curl -s http://localhost:8081/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Control Plane is healthy${NC}"
    else
        echo -e "${YELLOW}⚠️  Control Plane not responding (may need more time)${NC}"
    fi
    
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}🎉 Setup complete!${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "Test it:"
    echo "  curl http://localhost:8080/api/trigger/users"
    echo ""
    echo "View logs:"
    echo "  docker-compose logs -f        # Docker mode"
    echo "  tail -f *.log                 # Local mode"
    echo ""
}

main
