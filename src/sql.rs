pub const INSERT_ORDER: &'static str = "INSERT INTO orders 
(card_id,order_count,card_raw_price,card_cost,card_profit,design_cost,discount,customer_paid,ordered_at) 
VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9); ";

pub const INSERT_EXPENSE: &'static str = "INSERT INTO expenses2 
(kind,cost,description,issued_at) 
VALUES(?1,?2,?3,?4); ";

pub const CREATE_ORDER_TABLE: &'static str = "CREATE TABLE IF NOT EXISTS orders(
        id integer primary key,
        card_id text not null,
        order_count integer check(order_count>0),
        card_raw_price integer check(card_raw_price>=0),
        card_cost integer check(card_cost>=0),
        card_profit integer check(card_profit>=0),
        design_cost integer,
        discount integer,
        customer_paid integer,
        ordered_at text not null
    );";
pub const CREATE_EXPENSE_TABLE: &'static str = "CREATE TABLE IF NOT EXISTS expenses (
  id INTEGER PRIMARY KEY  NOT NULL,
  card INTEGER DEFAULT 0,
  ink INTEGER DEFAULT 0,
  printer INTEGER DEFAULT 0,
  maintanance INTEGER DEFAULT 0,
  description TEXT ,
  issued_at text NOT NULL
);";
pub const CREATE_CATEGORIES_TABLE :&str = "create table if not EXISTS categories (
  id integer primary key NOT NULL,
  title text NOT NULL,
  description text,
  parent integer
);";

pub const SHOW_EXPENSES:&str ="Select * from expenses2 where issued_at>=?1 and issued_at<=?2";

pub const ORDERS_EXPENSE_REPORT:&'static str = "select '+' as `income`,`raw-card-cost`,`paid`-`raw-card-cost`-`profit` as 'printer-ink',`profit`, `paid` as 'account balanec' from (
select 
  COALESCE(sum(orders.card_raw_price * orders.order_count),0) as 'raw-card-cost',
  COALESCE(sum(orders.card_profit * orders.order_count + orders.design_cost - orders.discount),0) as 'profit',
  COALESCE(sum(orders.customer_paid),0) as 'paid'
FROM
  orders
WHERE
	orders.ordered_at >=?1 AND orders.ordered_at <=?2
)

UNION ALL

SELECT
  '-' as `outcome`,
	COALESCE(sum( CASE WHEN kind = 'card' THEN cost ELSE 0 END ),0) as 'raw-card',
	COALESCE(sum( CASE WHEN kind = 'ink' OR kind = 'printer' OR kind = 'maintenance' THEN cost ELSE 0 END ),0) as 'ink+printer+maintain',
	0 as ' ',
	0 as '  '
FROM
	expenses2
WHERE
	expenses2.issued_at >=?1 AND expenses2.issued_at <=?2";

  pub const GET_ROOT_CATEGORIES:&str ="select * from categories where parent is null";
  pub const GET_SUB_CATEGORIES:&str="WITH RECURSIVE tree AS(
    select id, title from categoies where id = ?1
    union all
    select child.id, child.title from categories as child join tree where child.parent = tree.id

  ) Select * from tree;";