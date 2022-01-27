DROP TABLE public.logs CASCADE;
DROP TABLE public.log_tags CASCADE;

CREATE TABLE public.logs (
	id BIGSERIAL PRIMARY KEY,
	log_parts VARCHAR[],
	separators VARCHAR[],
	level SMALLINT,
	recorded_at timestamp WITHOUT TIME ZONE, -- partition key
	created_at TIMESTAMP WITHOUT TIME ZONE
);

CREATE TABLE public.log_tags (
	id BIGSERIAL PRIMARY KEY,
	log_id bigint REFERENCES public.logs(id),
	tag_name_id bigint REFERENCES public.tag_names(id),
	tag_value_id bigint REFERENCES public.tag_values(id),
	recorded_at timestamp without time zone -- allows to partition
);

CREATE EXTENSION IF NOT EXISTS btree_gin;

CREATE INDEX ON public.logs USING gin(created_at, log_parts);