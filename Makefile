.PHONY: help build up down restart logs test clean

help:
	@echo "Integration Platform - Available Commands"
	@echo "=========================================="
	@echo "  make build    - Build all Docker images"
	@echo "  make up       - Start all services"
	@echo "  make down     - Stop all services"
	@echo "  make restart  - Restart all services"
	@echo "  make logs     - View logs from all services"
	@echo "  make test     - Run test suite"
	@echo "  make clean    - Remove all containers and volumes"
	@echo ""

build:
	@echo "🔨 Building Docker images..."
	docker-compose build

up:
	@echo "🚀 Starting services..."
	docker-compose up -d
	@echo "⏳ Waiting for services to be ready..."
	@sleep 10
	@echo "✅ Services are up!"
	@echo ""
	@echo "Services:"
	@echo "  - Control Plane: http://localhost:8081"
	@echo "  - Data Plane:    http://localhost:8080"
	@echo "  - PostgreSQL:    localhost:5432"
	@echo ""
	@echo "Run 'make test' to test the services"

down:
	@echo "🛑 Stopping services..."
	docker-compose down

restart:
	@echo "🔄 Restarting services..."
	docker-compose restart
	@sleep 5
	@echo "✅ Services restarted!"

logs:
	docker-compose logs -f

test:
	@echo "🧪 Running tests..."
	@./test.sh

clean:
	@echo "🧹 Cleaning up..."
	docker-compose down -v
	@echo "✅ Cleanup complete!"
