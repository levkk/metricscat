-- Add migration script here
CREATE TABLE public.tag_names (
	id BIGSERIAL PRIMARY KEY,
	name VARCHAR UNIQUE
);

CREATE TABLE public.tag_values (
	id BIGSERIAL PRIMARY KEY,
	value VARCHAR UNIQUE
);

CREATE TABLE public.metric_tags (
	id BIGSERIAL PRIMARY KEY,
	metric_id bigint REFERENCES public.metrics(id),
	tag_name_id bigint REFERENCES public.tag_names(id),
	tag_value_id bigint REFERENCES public.tag_values(id),
	recorded_at timestamp without time zone -- allows to partition
);

CREATE INDEX ON public.metric_tags USING btree(tag_name_id, metric_id);

-- Typo from before
ALTER TABLE metrics RENAME COLUMN metric_id TO metric_name_id;