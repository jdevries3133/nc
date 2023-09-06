create extension if not exists "uuid-ossp";

-- Ensure the database and schema names here match the databaes and schema
-- name in the `.env` file.
create database nc;
create schema nc;
\c nc;


create table collection(
    id serial primary key,
    name varchar(255) not null
);

create table property_type(
    id serial primary key,
    name varchar(255) not null
);

insert into property_type (name) values
    ('boolean'), --------- 1
    ('int'), ------------- 2
    ('float'), ----------- 3
    ('string'), ---------- 4
    ('multi-string'), ---- 5
    ('date'), ------------ 6
    ('datetime') --------- 7
;

create table property(
    id serial primary key,
    name varchar(255) not null,

    type_id int not null references property_type(id),
    collection_id int not null references collection(id)
);

create table page(
    id serial primary key,
    title varchar(255) not null,

    collection_id int not null references collection(id)
);

-- This is factored out because we don't need to join this table for i.e,
-- list views
create table page_content(
    page_id int primary key references page(id) on delete cascade,
    content text not null
);

create table propval_bool(
    value boolean not null,

    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id) on delete cascade,
    primary key (page_id, prop_id)
);

create table propval_int(
    value bigint not null,

    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id) on delete cascade,
    primary key (page_id, prop_id)
);

create table propval_float(
    value float not null,

    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id) on delete cascade,
    primary key (page_id, prop_id)
);

create table propval_str(
    value varchar(511) not null,

    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id) on delete cascade,
    primary key (page_id, prop_id)
);

create table propval_date(
    value date not null,

    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id) on delete cascade,
    primary key (page_id, prop_id)
);

create table propval_datetime(
    value timestamp with time zone not null,

    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id),
    primary key (page_id, prop_id)
);

create table propval_multistr(
    id serial primary key,
    page_id int not null references page(id) on delete cascade,
    prop_id int not null references property(id) on delete cascade,

    unique(page_id, prop_id)
);

create table propval_multistr__value(
    value varchar(511) not null,
    propval_multistr_id int not null references propval_multistr(id) on delete cascade
);

-- Starter Data

insert into collection (name) values ('Default Collection');
insert into property (name, type_id, collection_id) values
    ('Sprint Number', 2, 1),
    ('Due Date', 6, 1),
    ('Completed', 1, 1)
;
insert into page (title, collection_id) values
    ('Build multi-string support', 1),
    ('Get started on git integration. This is a really long ticket with a long title; gee, so many words. I wonder if our layout can support this?', 1)
;
insert into propval_bool (value, page_id, prop_id) values
    (false, 1, 3),
    (false, 2, 3)
;
insert into propval_int (value, page_id, prop_id) values 
    (1, 1, 1),
    (2, 2, 1)
;
insert into propval_date (value, page_id, prop_id) values
    ('2023-09-01', 1, 2),
    ('2023-09-15', 2, 2)
;





















create table item(
    id serial primary key,
    title varchar(255) not null,
    is_completed boolean not null default false
);

insert into item (title, is_completed) values
    ('do the thing 1', false),
    ('do the thing 2', false),
    ('do the thing 3', false),
    ('do the thing 4', false),
    ('do the thing 5', false),
    ('do the thing 6', false),
    ('do the thing 7', false),
    ('do the thing 8', false),
    ('do the thing 9', false),
    ('do the thing 10', false),
    ('and the other thing is done', true)
;
