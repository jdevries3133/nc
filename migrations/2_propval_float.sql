insert into property (name, type_id, collection_id, "order") values
    ('Percentage', 3, 1, 4)
;

create table filter_float(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id) on delete cascade,
    value float not null
);

create table filter_float_range(
    id serial primary key,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id) on delete cascade,
    start float not null,
    "end" float not null
);
