create table filter_datetime(
    id serial not null,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id),

    value timestamp with time zone not null
);

create table filter_datetime_range(
    id serial not null,
    type_id int not null references filter_type(id),
    prop_id int not null references property(id),

    start timestamp with time zone not null,
    "end" timestamp with time zone not null
);
