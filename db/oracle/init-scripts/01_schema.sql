-- Schema for appuser in FREEPDB1
-- gvenzl/oracle-free runs these scripts as APP_USER automatically

CREATE TABLE customers (
    customer_id NUMBER         GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name        VARCHAR2(100)  NOT NULL,
    email       VARCHAR2(150)  NOT NULL UNIQUE,
    phone       VARCHAR2(20),
    city        VARCHAR2(100),
    created_at  TIMESTAMP      DEFAULT SYSTIMESTAMP
);

CREATE TABLE products (
    product_id  NUMBER         GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name        VARCHAR2(150)  NOT NULL,
    category    VARCHAR2(100),
    price       NUMBER(10, 2)  NOT NULL,
    stock_qty   NUMBER         DEFAULT 0
);

CREATE TABLE orders (
    order_id    NUMBER         GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    customer_id NUMBER         NOT NULL,
    order_date  TIMESTAMP      DEFAULT SYSTIMESTAMP,
    status      VARCHAR2(20)   DEFAULT 'pending' NOT NULL
                    CONSTRAINT chk_order_status
                    CHECK (status IN ('pending', 'confirmed', 'shipped', 'delivered', 'cancelled')),
    total       NUMBER(10, 2)  NOT NULL,
    CONSTRAINT fk_orders_customer FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
);

CREATE TABLE order_items (
    item_id     NUMBER         GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    order_id    NUMBER         NOT NULL,
    product_id  NUMBER         NOT NULL,
    quantity    NUMBER         NOT NULL,
    unit_price  NUMBER(10, 2)  NOT NULL,
    CONSTRAINT fk_items_order   FOREIGN KEY (order_id)   REFERENCES orders(order_id),
    CONSTRAINT fk_items_product FOREIGN KEY (product_id) REFERENCES products(product_id)
);
