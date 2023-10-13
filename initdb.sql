create extension if not exists "uuid-ossp";

-- Ensure the database and schema names here match the databaes and schema
-- name in the `.env` file.
create database nc;
create schema nc;
\c nc;

create table users(
    id serial primary key,
    username varchar(255) unique not null,
    email varchar(255) unique not null,
    salt varchar(255) not null,
    digest varchar(255) not null
);

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
    "order" smallint not null,

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

create table filter_type(
    id serial primary key,
    name varchar(255) not null
);

insert into filter_type (name) values
    ('Exactly Equals'),  ------ 1
    ('Does not Equal'),  ------ 2
    ('Is Greater Than'), ------ 3
    ('Is Less Than'), --------- 4
    ('Is Inside Range'), ------ 5
    ('Is Not Inside Range'), -- 6
    ('Is Empty') -------------- 7
;

create table filter_bool(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int unique not null references property(id) on delete cascade,
    value boolean not null
);

create table filter_int(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id) on delete cascade,
    value bigint not null
);

create table filter_int_range(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id) on delete cascade,
    start bigint not null,
    "end" bigint not null
);

create table sort_type(
    id serial primary key,
    name varchar(255) not null
);

insert into sort_type (name) values
    ('Ascending'),
    ('Descending')
;

alter table collection add column sort_by_prop_id int references property(id);

alter table collection add column sort_type_id int references sort_type(id);


-- Starter Data

insert into collection (name) values ('Default Collection');
insert into property (name, type_id, collection_id, "order") values
    ('Sprint Number', 2, 1, 1),
    ('Completed', 1, 1, 2),
    ('Age', 2, 1, 3)
;
insert into page (title, collection_id) values
    ('Build multi-string support', 1),
    ('Get started on git integration. This is a really long ticket with a long title; gee, so many words. I wonder if our layout can support this?', 1),
    ('Do the thing!', 1),
    ('Oh, and the other thing too!!', 1)
;
insert into propval_int (value, page_id, prop_id) values 
    (3, 1, 1),
    (2, 2, 1)
;
insert into propval_bool (value, page_id, prop_id) values
    (true, 1, 2),
    (false, 2, 2)
;

insert into filter_bool (type_id, prop_id, value) values (1, 2, true);
insert into filter_int (type_id, prop_id, value) values (3, 1, 1);
insert into filter_int_range (type_id, prop_id, start, "end") values
    (5, 1, 0, 10)
;
