create DATABASE test;

use test;

CREATE LOGIN testuser WITH PASSWORD = 'test@Password123';

CREATE USER testuser FOR LOGIN testuser;



ALTER ROLE db_owner ADD MEMBER testuser;

-- drop table dbo.customers;

CREATE TABLE dbo.customers (
    customer_id INT IDENTITY(1, 1) PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    email       VARCHAR(150) NOT NULL UNIQUE,
    phone       VARCHAR(20),
    city        VARCHAR(100),
    created_at  DATETIME DEFAULT getdate()
);




select * from dbo.customers;

CREATE TABLE dbo.products (
    product_id  INT IDENTITY(1, 1) PRIMARY KEY,
    name        VARCHAR(150) NOT NULL,
    category    VARCHAR(100),
    price       DECIMAL(10, 2) NOT NULL,
    stock_qty   INT DEFAULT 0
);

CREATE TABLE dbo.orders (
    order_id    INT IDENTITY(1, 1) PRIMARY KEY,
    customer_id INT NOT NULL,
    order_date  DATETIME DEFAULT CURRENT_TIMESTAMP,
    status      NVARCHAR(20) NOT NULL CHECK (status IN ('pending', 'confirmed', 'shipped', 'delivered', 'cancelled')) DEFAULT 'pending',
    total       DECIMAL(10, 2) NOT NULL,
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
);

CREATE TABLE dbo.order_items (
    item_id     INT IDENTITY(1, 1) PRIMARY KEY,
    order_id    INT NOT NULL,
    product_id  INT NOT NULL,
    quantity    INT NOT NULL,
    unit_price  DECIMAL(10, 2) NOT NULL,
    FOREIGN KEY (order_id)   REFERENCES orders(order_id),
    FOREIGN KEY (product_id) REFERENCES products(product_id)
);
