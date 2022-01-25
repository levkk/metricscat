-- Add migration script here
CREATE TABLE public.logs (
	id BIGSERIAL PRIMARY KEY,
	log_line TEXT,
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
)