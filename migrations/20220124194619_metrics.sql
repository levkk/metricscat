-- Add migration script here
CREATE TABLE public.metric_names (
    id BIGSERIAL PRIMARY KEY,
    name character varying UNIQUE
);

CREATE TABLE public.metrics (
    id BIGSERIAL PRIMARY KEY,
    metric_id bigint REFERENCES public.metric_names(id),
    value double precision,
    recorded_at timestamp without time zone -- todo: partition by this
);

CREATE INDEX ON public.metrics USING btree(recorded_at, metric_id);