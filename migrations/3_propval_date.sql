insert into property (name, type_id, collection_id, "order") values
    ('Birthday', 6, 1, 5)
;

create table filter_date(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id) on delete cascade,
    value date not null
);

create table filter_date_range(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id) on delete cascade,
    start date not null,
    "end" date not null
);
