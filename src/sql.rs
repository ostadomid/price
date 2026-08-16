pub const INSERT_ORDER: &'static str = "INSERT INTO orders 
(card_id,order_count,card_raw_price,card_cost,card_profit,design_cost,discount,customer_paid,ordered_at) 
VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9); ";

pub const CREATE_ORDER_TABLE: &'static str = "CREATE TABLE IF NOT EXISTS orders(
        id integer primary key,
        card_id text not null,
        order_count integer check(order_count>0),
        card_raw_price integer check(card_cost>0),
        card_cost integer check(card_cost>0),
        card_profit integer check(card_profit>0),
        design_cost integer,
        discount integer,
        customer_paid integer,
        ordered_at text not null
    );";
