-- 100 deterministic examples produced by the JOB differential-test generator.
-- Each block is isolated in a transaction so this entire file can be run at once.
-- Remove its ROLLBACK to keep a particular database for interactive queries.

-- ============================================================================
-- Generated database 001/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', NULL, 0, '', '', '');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', 0, '', '', '', '', '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', '', 0, '', '', '');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '0', '', 1, 2005, 0, '', 0, 0, 0, '', '');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '', '', '', '', '', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', '', 1, 0, '', 0, 0, 0, '', '');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, '(voice) (uncredited)', 0, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', '');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '', '');
ROLLBACK;

-- ============================================================================
-- Generated database 002/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', NULL, 146, NULL, '_', 'GUDUd4dGFgUg'),
(2, '''IE', NULL, NULL, 'AG', NULL, 'j'),
(3, '88qwqqwMw', '[us]', 867, 'q''', NULL, 'NIL');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'None'),
(2, 'marvel-cinematic-universe', 'XSS'),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', 'vvvIvvI6v', NULL, 'z', 'arYa', NULL, 'nil', ''''''''''''''),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, 'eTjP', NULL, NULL, 'True');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, '__proto__', NULL),
(2, 'Other Character', NULL, NULL, NULL, '0c', '0');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'undefined', NULL, 2, NULL, 5897, 'NxJNlKxKlx', 163, NULL, NULL, NULL, NULL),
(2, 'nHgnn', '', 2, NULL, NULL, 'MJMM', 48, NULL, NULL, 'QD', '8r88p8'),
(3, 'CD9%C%8%', 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', 1, 2009, 80, 'DWvE3 3 oWE__ E', 4020, NULL, 912, NULL, '');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '', '2a22', NULL, '5 _ CCD__', '', 'rrcrrrrrr'),
(2, 2, 'true', '', NULL, 'ppp', NULL, 'YK'),
(3, 2, '__w_', NULL, 'X', NULL, '-Infinity', 'qQQ');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'jih', 'RaM', 2, 264, NULL, 167, NULL, 439, 'r', '__proto__'),
(2, 1, 'D', 'else', 2, 735, NULL, 850, NULL, 305, NULL, 'ZuZ'),
(3, 1, '', 'Cw', 1, 1737, NULL, NULL, 3048, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, NULL, '(voice)', NULL, 1),
(2, 2, 2, 2, NULL, 821, 1),
(3, 1, 3, NULL, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, 'NYY'),
(2, 1, 1, 1, 'wcw33c');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 2, 'Bulgaria', NULL),
(3, 2, 1, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, 'RRRRRR', 'UYwUwYwYUwUY'),
(2, 2, 2, 'nil', 'TRUE');
ROLLBACK;

-- ============================================================================
-- Generated database 003/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'NUL', NULL, 1019, 'k', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, 'aa', NULL, NULL, NULL, NULL),
(2, 'Other Person', '', 255, NULL, NULL, NULL, '0', '7'),
(3, 'Other Person', NULL, 1322, 'ZRRsZRRZ', 'TRUE', NULL, 'tB', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'aaaa', 'Tu u   ', 2, NULL, 297, NULL, NULL, 15, 69, 'cpcapvSaaccapvavc', 'YO'),
(2, 'Inf', '999999999999999999999999999999', 1, NULL, 415, NULL, NULL, NULL, NULL, NULL, 'Uj'),
(3, 'Xi', 'nErVr', 2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '999999999999999999999999999999');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'McMcc', 'z5zz5z5', 'vZvZvv ZZZ  vZ', NULL, 'HHdH', NULL),
(2, 2, '%ccw%rM%rmmMc%%ccbrcMpcww%', 'V2wZ', NULL, NULL, '%1', '0');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Inf', '--', 1, 1024, NULL, NULL, NULL, 30, '1uTM4', 'CnaC'),
(2, 1, 'Bw', '-Infinity', 1, 4553, NULL, 1914, 454, NULL, '', NULL),
(3, 3, '', 'True', 2, 605, NULL, 4096, 775, NULL, 'm', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 2, 1, NULL, 4096, 1),
(2, 2, 1, 1, NULL, 1024, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 1),
(2, NULL, 3, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 1, 1, 1, 'TY');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 2),
(2, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 1, 1),
(2, 3, 1, 1),
(3, 1, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'c_jc', '0ZAZ');
ROLLBACK;

-- ============================================================================
-- Generated database 004/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'vvc', '[de]', NULL, NULL, NULL, 'k2i22'),
(2, 'b', '[us]', 6, '999999999999999999999999999999', 'pdqoTT2', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', 'S'),
(3, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'OJqTJ', NULL, NULL, 'XXL22XD22PPX2L22LXnSnPdnLn'),
(2, 'Downey Jr., Robert', NULL, 34, NULL, '', 'M0M', NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 343, NULL, 'www', '3iM'),
(2, 'Other Character', NULL, NULL, '-Infinity', NULL, 'none');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'yCCB-yC4CCvB-', 'RZ6T', 1, NULL, 8192, 'undefined', NULL, NULL, 63, '', NULL),
(2, 'NaN', 'eP', 1, 2005, 6694, NULL, 289, 1021, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'V', 'BBjBjl', NULL, 'Ji', 'T', 'else'),
(2, 1, '', NULL, 'asj', NULL, 'else', NULL),
(3, 1, '1e100', NULL, NULL, 'V', 'nil', 'null');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '77373', NULL, 2, 229, NULL, 5, NULL, NULL, '', 'i');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(uncredited)', 256, 1),
(2, 2, 1, NULL, NULL, NULL, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, NULL, 1, 1),
(3, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 2, 'l');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', NULL),
(2, 2, 2, 'USA', ''),
(3, 1, 3, 'Bulgaria', 'GlTj');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL),
(2, 2, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, 'fJiCosJoSiyCSCCCiiCfySsoCCfJis', 'NUL'),
(2, 1, 3, '-Infinity', NULL),
(3, 2, 1, 'cd ', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 005/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', NULL, NULL, '', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'D8');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 31, 'yy', NULL, 'Infinity', 'rw', 'Scunthorpe');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 16, 'nil', 'ZZ', NULL),
(2, 'Voice Character', '', 88, 'yyRyRRyyyRRROyRyyyOOORy', NULL, 'null');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '0x0QQ00Q000Q', NULL, 2, 2006, NULL, NULL, NULL, NULL, NULL, '8', 'LLL'),
(2, '8E', '- w ww  5w -w5', 2, NULL, NULL, NULL, 31, 2, NULL, 'z3ZzZ3ZzZZ3zZZ3ZZ3zZ33', 'NaN'),
(3, 'y1', '', 2, NULL, NULL, '1e100', 7, 8192, 2, NULL, 'true');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'Infinity', NULL, NULL, NULL, 'NIL', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'ZZZZZ', '', 2, NULL, NULL, 64, NULL, NULL, '-NN-N', 'uuSmmbSXSSmuXSbbSXuSS');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, NULL, '(uncredited)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 1, 1, NULL),
(2, 3, 1, 1, NULL),
(3, 2, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 3, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', 'DlRDgl');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '__proto__', 'LLR'),
(2, 1, 2, 'NUL', NULL),
(3, 1, 1, '0', 'Z');
ROLLBACK;

-- ============================================================================
-- Generated database 006/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'nrxprUxiU', NULL, 5518, 'ASlGGMKMMgl', NULL, 'SL00v'),
(2, '4e44', NULL, NULL, NULL, NULL, NULL),
(3, 'INF', '[us]', 323, '1', 'wF', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'character-name-in-title', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '%TBQM', 6040, 'CCC', NULL, NULL, NULL, 'true'),
(2, 'Other Person', 'cc', NULL, '9ittjijttOtW1tOOi99tiW', 'q', NULL, 'WdWqW', 'aaIxHaaaIaI'),
(3, 'Other Person', 'ccccc', 41, 'xxbghNxNxghgghbgNgg3b33N3gNb3g43', NULL, NULL, NULL, 'CJCvCCC');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 1113, '1Kuj_3j_uuK1j_jj1v', NULL, NULL),
(2, 'Voice Character', NULL, 85, NULL, 'SNTpppLNLNTLTSLNpL', NULL),
(3, 'Voice Character', 'false', 540, '''A', 'qqq', 'True');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '999999999999999999999999999999', NULL, 1, 2011, 2931, NULL, NULL, 381, NULL, 'EnEE', NULL),
(2, 'w-88-w8aWw', 'KQ__QS_SN', 1, 2012, 1465, 'qDS', NULL, NULL, NULL, 'obbovo', NULL),
(3, 'CC4tCw_9', '7EM7VEV777MEME7V7E7EVEVVEVEVVEEE', 1, NULL, NULL, NULL, 485, NULL, NULL, 'ttttttttttt', 'null');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'qqqvrrqrvvvq', 'H%H6H', '4l', NULL, 'qDDvDq', NULL),
(2, 3, 'nI', NULL, 'x', NULL, 'ug  Q cgcWc 2W', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '88''Xq', NULL, 1, 97, NULL, NULL, 410, NULL, 'ZlllKKOTK''2TfTEKOfEOET''''', 'TTTx'),
(2, 1, '8jFK--vFKzzFjK8KzFS8zz8K-jj', NULL, 1, NULL, 'wwuwwPBuPPwPwGGGwwBGBwBP', NULL, NULL, 1447, '%B%', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 3, 3, '(voice) (uncredited)', 273, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 3, NULL),
(2, 2, 2, 2, NULL),
(3, 1, 3, 3, 'true');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'USA', NULL),
(2, 2, 1, 'USA', 'FFFF3FFFF');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '8.0', NULL),
(2, 3, 1, '8.0', NULL),
(3, 1, 2, '8.0', '');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1),
(2, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 3, 1, '', 'UUUUUU'),
(2, 2, 2, 'VDNHVHzHDVDz', '9pvvr');
ROLLBACK;

-- ============================================================================
-- Generated database 007/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '999999999999999999999999999999', '[de]', 816, 'nA11nIntjtIgA', 'PPP', 'f ');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, NULL, '__dict__', 'd1--M-M--lMV1l', 'null', NULL),
(2, 'Downey Jr., Robert', 'NUL', NULL, NULL, NULL, 'TRUE', NULL, 'CCCCCC');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '''''''', 0, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'false', 'None', 2, 2009, 127, '3j3j4', 5299, 2, NULL, NULL, 'G55GqGyqTU5GGDqU');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'rr', 'FFFFFFFFFFFFFFFF', NULL, NULL, 'FALSE', NULL),
(2, 1, 'BBbkxkxMBxMkMxkBkB3', '0', '', NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'ic', NULL, 1, NULL, 'ZB', 227, 94, 256, 'b33bb3N', NULL),
(2, 1, 'rE5ZfMCrffMZf5', '333U', 1, NULL, ' K r', 511, 127, NULL, NULL, 'D4');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, 1, '(voice)', NULL, 3),
(2, 1, 1, 1, NULL, 6, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', NULL),
(2, 1, 1, 'USA', '0'),
(3, 1, 2, 'USA', 'D');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', '0'),
(2, 1, 1, '4.0', 'y'),
(3, 1, 1, '4.0', 'FFMMFMMMFMQMMM');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '%Hb''bHHbH%%Wcb%t''t', NULL),
(2, 2, 1, 'Scunthorpe', 'OMvOv99C%vC%%O%'),
(3, 1, 1, '', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 008/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'TRUE', NULL, 6321, 'FTTTT', 'SSK42KSaS', 'wwwIww'),
(2, 'TTTT', '[de]', NULL, NULL, 'False', 'then'),
(3, '', NULL, 3931, '%II', '00vn0vRDnwDnAoDAv', 'W');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', NULL, 'ILyy', '2', 'nnnnn', '''''''w''cHHw', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'NULL', 1125, 'nnWryQQ_', NULL, 'NUL');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'cccc44c44cf4 cc4fc f444f cfccfcc', 'OGGOOGOGGGOGO', 2, 2012, NULL, NULL, NULL, 9999, NULL, ' a4F  ', NULL),
(2, 'YY', 'NaN', 1, 2006, NULL, NULL, 0, NULL, 656, NULL, 'FALSE'),
(3, '-N-NNN', 'ii999q99qq9iq', 2, 2007, 439, '0', NULL, 5712, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'S4S444SS4S44S4', NULL, NULL, 'o-fo2L--Lfo8-G', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'FALSE', 'dB6', 1, 1096, NULL, NULL, NULL, NULL, '', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', NULL, 1),
(2, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, NULL, 3, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 1, 'None'),
(2, 2, 3, 2, '070 '),
(3, 2, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 3, 2, '8.0', 'R%');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 3, 2),
(2, 3, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, 'Tt', NULL),
(2, 1, 2, 'Scunthorpe', NULL),
(3, 1, 2, 'FFFFFFFFFFFFFF', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 009/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'true', NULL, 8, NULL, 'NKd', 'False'),
(2, 'EEXkkXiEXiX', NULL, 2048, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 198, NULL, 'P', NULL, '0', 'None');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '     m mm ', 4092, NULL, 'VYtYVh', NULL),
(2, 'Other Character', 'nhnRJR', 2736, NULL, 'false', NULL),
(3, 'Voice Character', 'ooooooooooo', NULL, 'SSSY', 'T09T9', 'else');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '25f2qfq5_', 'o', 3, NULL, NULL, 'Yf', 605, 1510, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'i6i8', NULL, NULL, 'Inf', '__dict__', NULL),
(2, 1, 'False', NULL, 'ZZ%%%Z', NULL, '33o', 'qz'),
(3, 1, '1e100', '0', 'INF', NULL, NULL, 'YccNv');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Infinity', '77777', 3, NULL, 'FALSE', 103, 196, NULL, '', '7b7bdb'),
(2, 1, '777uS-S7SuSuuSS7S', '', 2, 127, '', NULL, NULL, 4576, NULL, 'false');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, 'TRUE'),
(3, 1, 2, 1, '99999Y99');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'USA', NULL),
(2, 1, 3, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', 'MyMyJMyMJJMJyy'),
(2, 1, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'Inf', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 010/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'maXXagXXjmXXjg', NULL, 462, 'uu', NULL, 'OYY'),
(2, 'true', NULL, NULL, NULL, '1V%1VV%%1', NULL),
(3, 'YjY', '[de]', 30, NULL, 'FFssQoEFooEFo', '0');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'aagna%1gga1a'),
(2, 'hero-sequel', 'True');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references'),
(3, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 485, 'cFwwFF', NULL, NULL, '', '7'),
(2, 'Other Person', NULL, 402, 'v%v', 'VVVVVVVVVVVVV', NULL, 'none', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'NYC', NULL, NULL, NULL, NULL),
(2, 'Other Character', 'r0Ah', 10000, 'null', NULL, 'A');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'TRUE', 'NaN', 1, NULL, NULL, NULL, 1632, NULL, 5529, NULL, NULL),
(2, 'FFF', 'True', 1, NULL, 384, NULL, NULL, NULL, NULL, '', NULL),
(3, '8W', 'B3300', 1, 2010, NULL, NULL, 3378, 214, 9306, 'cK', 'H');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'YYKdoodd', 'cccb4i', NULL, NULL, 'TQlQTx', 'rfYrrw%wRYyffw'),
(2, 2, '0', NULL, NULL, 'vr', 'null', NULL),
(3, 2, 'sS', NULL, 'FALSE', 'ilii1H-i1', '77', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'gggggggggggggg', NULL, 1, NULL, NULL, 5929, NULL, 2047, 'FALSE', 'True'),
(2, 3, '66ggg66', 'X', 1, NULL, 'K-KK-aKKaK--a-KaKa-KK--Ka-a-KKKa', NULL, 228, NULL, NULL, 'Q2ff2'),
(3, 1, 'i2ciicc2ci', NULL, 1, NULL, 'FH''2', 1197, NULL, NULL, '00', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 3, NULL, '(voice)', 10000, 1),
(2, 2, 3, 2, '(voice) (uncredited)', NULL, 2),
(3, 2, 2, 2, NULL, 4466, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, NULL, 1, 1),
(3, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 1, NULL),
(2, 2, 1, 1, NULL),
(3, 1, 1, 1, 'FALSE');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'Bulgaria', '95l59llj5'),
(2, 2, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', 'T2'),
(2, 2, 2, '4.0', NULL),
(3, 1, 1, '4.0', 'lllPl');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 3),
(2, 3, 2, 2),
(3, 1, 3, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, 'vD4FF', NULL),
(2, 2, 1, 'Inf', NULL),
(3, 1, 1, 'TRUE', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 011/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'maXXagXXjmXXjg', NULL, 462, 'uu', NULL, 'OYY'),
(2, 'true', NULL, NULL, NULL, '1V%1VV%%1', NULL),
(3, 'YjY', '[de]', 30, NULL, 'FFssQoEFooEFo', '0');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'aagna%1gga1a'),
(2, 'hero-sequel', 'True');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references'),
(3, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 485, 'cFwwFF', NULL, NULL, '', '7'),
(2, 'Other Person', NULL, 402, 'v%v', 'VVVVVVVVVVVVV', NULL, 'none', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'NYC', NULL, NULL, NULL, NULL),
(2, 'Other Character', 'r0Ah', 10000, 'null', NULL, 'A');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'TRUE', 'NaN', 1, NULL, NULL, NULL, 1632, NULL, 5529, NULL, NULL),
(2, 'FFF', 'True', 1, NULL, 384, NULL, NULL, NULL, NULL, '', NULL),
(3, '8W', 'B3300', 1, 2010, NULL, NULL, 3378, 214, 9306, 'cK', 'H');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'YYKdoodd', 'cccb4i', NULL, NULL, 'TQlQTx', 'rfYrrw%wRYyffw'),
(2, 2, '0', NULL, NULL, 'vr', 'null', NULL),
(3, 2, 'sS', NULL, 'FALSE', 'ilii1H-i1', '77', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'gggggggggggggg', NULL, 1, NULL, NULL, 5929, NULL, 2047, 'FALSE', 'True'),
(2, 3, '66ggg66', 'X', 1, NULL, 'K-KK-aKKaK--a-KaKa-KK--Ka-a-KKKa', NULL, 228, NULL, NULL, 'Q2ff2'),
(3, 1, 'aagna%1gga1a', NULL, 1, NULL, 'FH''2', 1197, NULL, NULL, '00', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 3, NULL, '(voice)', 10000, 1),
(2, 2, 3, 2, '(voice) (uncredited)', NULL, 2),
(3, 2, 2, 2, NULL, 4466, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, NULL, 1, 1),
(3, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 1, NULL),
(2, 2, 1, 1, NULL),
(3, 1, 1, 1, 'FALSE');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'Bulgaria', '95l59llj5'),
(2, 2, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', 'T2'),
(2, 2, 2, '4.0', NULL),
(3, 1, 1, '4.0', 'lllPl');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 3),
(2, 3, 2, 2),
(3, 1, 3, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, 'vD4FF', NULL),
(2, 2, 1, 'Inf', NULL),
(3, 1, 1, 'TRUE', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 012/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'maXXagXXjmXXjg', NULL, 462, 'uu', NULL, 'OYY'),
(2, 'true', NULL, NULL, NULL, '1V%1VV%%1', NULL),
(3, 'YjY', '[de]', 30, NULL, 'FFssQoEFooEFo', '0');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'aagna%1gga1a'),
(2, 'hero-sequel', 'True');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references'),
(3, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 485, 'cFwwFF', NULL, NULL, '', '7'),
(2, 'Other Person', NULL, 402, 'v%v', 'VVVVVVVVVVVVV', NULL, 'none', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'NYC', NULL, NULL, NULL, NULL),
(2, 'Other Character', 'r0Ah', 10000, 'null', NULL, 'A');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'TRUE', 'NaN', 1, NULL, NULL, NULL, 1632, NULL, 5529, NULL, NULL),
(2, 'FFF', 'True', 1, NULL, 384, NULL, NULL, NULL, NULL, '', NULL),
(3, '8W', 'B3300', 1, 2010, NULL, NULL, 3378, 214, 9306, 'cK', 'H');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'YYKdoodd', 'cccb4i', NULL, NULL, 'TQlQTx', 'rfYrrw%wRYyffw'),
(2, 2, '0', NULL, NULL, 'vr', 'null', NULL),
(3, 2, 'sS', NULL, 'FALSE', 'ilii1H-i1', 'FFF', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'gggggggggggggg', NULL, 1, NULL, NULL, 5929, NULL, 2047, 'FALSE', 'True'),
(2, 3, '66ggg66', 'X', 1, NULL, 'K-KK-aKKaK--a-KaKa-KK--Ka-a-KKKa', NULL, 228, NULL, NULL, 'Q2ff2'),
(3, 1, 'i2ciicc2ci', NULL, 1, NULL, 'FH''2', 1197, NULL, NULL, '00', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 3, NULL, '(voice)', 10000, 1),
(2, 2, 3, 2, '(voice) (uncredited)', NULL, 2),
(3, 2, 2, 2, NULL, 4466, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, NULL, 1, 1),
(3, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 1, NULL),
(2, 2, 1, 1, NULL),
(3, 1, 1, 1, 'FALSE');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'Bulgaria', '95l59llj5'),
(2, 2, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', 'T2'),
(2, 2, 2, '4.0', NULL),
(3, 1, 1, '4.0', 'lllPl');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 3),
(2, 3, 2, 2),
(3, 1, 3, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, 'vD4FF', NULL),
(2, 2, 1, 'Inf', NULL),
(3, 1, 1, 'TRUE', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 013/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '333333333333333333333333', NULL, 4858, NULL, 'Vogog', 'L77LLL'),
(2, 'TRUE', NULL, NULL, 's', 'pOUggg0ggU00', 'Mnnzcc'),
(3, 'if', NULL, 420, '0', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''''),
(2, 'marvel-cinematic-universe', '0'),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 759, NULL, NULL, NULL, 'j', '00');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 2479, NULL, '7', NULL),
(2, 'Voice Character', NULL, 4096, NULL, 'OiSOiO', NULL),
(3, 'Voice Character', 'a00a9yay', 487, '99ri5r5YrBY9', '', '  ');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6-6dDQ-NQ', NULL, 1, NULL, 3040, NULL, 152, NULL, NULL, NULL, 'c-b'),
(2, '8', 'NUL', 1, 2011, 7119, '__dict__', NULL, NULL, 457, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '999999999999999999999999999999', NULL, NULL, 'c0d', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 1, 42, NULL, NULL, 285, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(uncredited)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, NULL, 3, 3),
(3, 1, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 1, 'HcK33F2HcSSl3S23SlF'),
(2, 1, 1, 2, 'else'),
(3, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL),
(2, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 3),
(2, 2, 1, 3),
(3, 1, 2, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'yiiiiRiiRRyRyiiRy', 'Q-m''m''Q-''m'),
(2, 1, 1, 'x1Is', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 014/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '333333333333333333333333', NULL, 4858, NULL, 'Vogog', 'L77LLL'),
(2, 'TRUE', NULL, NULL, 's', 'pOUggg0ggU00', 'Mnnzcc'),
(3, 'if', '[ru]', NULL, '0', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''''),
(2, 'marvel-cinematic-universe', '0'),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 759, NULL, NULL, NULL, 'j', '00');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 2479, NULL, '7', NULL),
(2, 'Voice Character', NULL, 4096, NULL, 'OiSOiO', NULL),
(3, 'Voice Character', 'a00a9yay', 487, '99ri5r5YrBY9', '', '  ');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6-6dDQ-NQ', NULL, 1, NULL, 3040, NULL, 152, NULL, NULL, NULL, 'c-b'),
(2, '8', 'NUL', 1, 2011, 7119, '__dict__', NULL, NULL, 457, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '999999999999999999999999999999', NULL, NULL, 'c0d', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 1, 42, NULL, NULL, 285, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(uncredited)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, NULL, 3, 3),
(3, 1, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 1, 'HcK33F2HcSSl3S23SlF'),
(2, 1, 1, 2, 'else'),
(3, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL),
(2, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 3),
(2, 2, 1, 3),
(3, 1, 2, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'yiiiiRiiRRyRyiiRy', 'Q-m''m''Q-''m'),
(2, 1, 1, 'x1Is', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 015/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '333333333333333333333333', NULL, 4858, NULL, 'Vogog', 'L77LLL'),
(2, 'TRUE', NULL, NULL, 's', 'pOUggg0ggU00', 'Mnnzcc'),
(3, 'if', NULL, 420, '0', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''''),
(2, 'marvel-cinematic-universe', '0'),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 759, NULL, NULL, NULL, 'j', '00');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 2479, NULL, '7', NULL),
(2, 'Voice Character', NULL, 4096, NULL, 'OiSOiO', NULL),
(3, 'Voice Character', 'a00a9yay', 487, 'pOUggg0ggU00', '', '  ');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6-6dDQ-NQ', NULL, 1, NULL, 3040, NULL, 152, NULL, NULL, NULL, 'c-b'),
(2, '8', 'NUL', 1, 2011, 7119, '__dict__', NULL, NULL, 457, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '999999999999999999999999999999', NULL, NULL, 'c0d', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 1, 42, NULL, NULL, 285, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(uncredited)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, NULL, 3, 3),
(3, 1, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 1, 'HcK33F2HcSSl3S23SlF'),
(2, 1, 1, 2, 'else'),
(3, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL),
(2, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 3),
(2, 2, 1, 3),
(3, 1, 2, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'yiiiiRiiRRyRyiiRy', 'Q-m''m''Q-''m'),
(2, 1, 1, 'x1Is', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 016/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '333333333333333333333333', NULL, 4858, NULL, 'Vogog', 'L77LLL'),
(2, 'TRUE', NULL, NULL, 's', 'pOUggg0ggU00', 'Mnnzcc'),
(3, 'if', NULL, 420, '0', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''''),
(2, 'marvel-cinematic-universe', '0'),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 759, NULL, NULL, NULL, 'j', '00');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 2479, NULL, '7', NULL),
(2, 'Voice Character', NULL, 4096, NULL, 'OiSOiO', NULL),
(3, 'Voice Character', 'a00a9yay', 487, '99ri5r5YrBY9', '', '  ');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6-6dDQ-NQ', NULL, 1, NULL, 3040, NULL, 152, NULL, NULL, NULL, 'c-b'),
(2, '8', 'NUL', 1, 2011, 7119, '__dict__', NULL, NULL, 457, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '999999999999999999999999999999', NULL, NULL, 'c0d', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 1, 42, NULL, NULL, 285, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(uncredited)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, NULL, 3, 3),
(3, 1, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 1, 'HcK33F2HcSSl3S23SlF'),
(2, 1, 1, 2, 'else'),
(3, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL),
(2, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 3),
(2, 2, 1, 3),
(3, 1, 2, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'yiiiiRiiRRyRyiiRy', 'Q-m''m''Q-''m'),
(2, 1, 1, 'x1Is', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 017/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '333333333333333333333333', NULL, 4858, NULL, 'Vogog', 'L77LLL'),
(2, 'TRUE', NULL, NULL, 's', 'pOUggg0ggU00', 'Mnnzcc'),
(3, 'if', NULL, 420, '0', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''''),
(2, 'marvel-cinematic-universe', '0'),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 759, NULL, NULL, NULL, 'j', '00');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 2479, NULL, '7', NULL),
(2, 'Voice Character', NULL, 4096, NULL, 'OiSOiO', NULL),
(3, 'Voice Character', 'a00a9yay', 487, '99ri5r5YrBY9', '', '  ');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6-6dDQ-NQ', NULL, 1, NULL, 3040, NULL, 152, NULL, NULL, NULL, 'c-b'),
(2, '8', 'NUL', 1, 2011, 7119, '__dict__', NULL, NULL, 457, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'NUL', NULL, NULL, 'c0d', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 1, 42, NULL, NULL, 285, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(uncredited)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, NULL, 3, 3),
(3, 1, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 1, 'HcK33F2HcSSl3S23SlF'),
(2, 1, 1, 2, 'else'),
(3, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL),
(2, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 3),
(2, 2, 1, 3),
(3, 1, 2, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'yiiiiRiiRRyRyiiRy', 'Q-m''m''Q-''m'),
(2, 1, 1, 'x1Is', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 018/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'COM1', '[de]', NULL, NULL, 'F', 'KzKKKzz'),
(2, 'Ruvv11uvvvuR1v1RR1RRu1', NULL, 7706, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 8999, 'False', 'F3FDz', 'n2sglgDs', NULL, 'o'),
(2, 'Downey Jr., Robert', NULL, 640, 'g', 'v6vIbb6b2WvVI6', NULL, 'EpEw5Aw', 'FzzzzzzF');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, 'O2ix', 'eee', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-QCCQ', NULL, 1, NULL, 1685, NULL, 3166, NULL, 276, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, '7f7fff', NULL, 'QRRXLL', 'xv'),
(2, 1, 'i5''Uv5U''v''UU''vi5''i5UU', NULL, NULL, 'etjU', 'PPQQ-88--Q8-8PQ-Q--Q--8-8P8Q8-PP', 'false'),
(3, 2, 'O LEUPEL', NULL, '2', 'S', 'none', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'B_gQ6kAQ_', '7c7cc7c5cc757', 1, 2524, 'nil', NULL, NULL, 575, NULL, 'tDdDdAQ'),
(2, 1, '_', NULL, 1, 5906, 'FALSE', NULL, 4095, NULL, 'TTTJJTTThTJJhJ', 'GGyGysGGssGsy'),
(3, 1, 'then', 'LPT1', 1, 218, NULL, 28, 1751, 896, NULL, '0');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, 2785, 1),
(2, 1, 1, 1, NULL, 532, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', '999999999999999999999999999999'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', 'J');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'c 8', '2222222222');
ROLLBACK;

-- ============================================================================
-- Generated database 019/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'COM1', '[de]', NULL, NULL, 'F', 'KzKKKzz'),
(2, 'Ruvv11uvvvuR1v1RR1RRu1', NULL, 7706, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 8999, 'False', 'F3FDz', 'n2sglgDs', NULL, 'o'),
(2, 'Downey Jr., Robert', NULL, 640, 'g', 'v6vIbb6b2WvVI6', NULL, 'EpEw5Aw', 'FzzzzzzF');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, 'O2ix', 'eee', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-QCCQ', NULL, 1, NULL, 1685, NULL, 3166, NULL, 276, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, '7f7fff', NULL, 'QRRXLL', 'xv'),
(2, 1, 'i5''Uv5U''v''UU''vi5''i5UU', NULL, NULL, 'etjU', 'PPQQ-88--Q8-8PQ-Q--Q--8-8P8Q8-PP', 'false'),
(3, 2, 'O LEUPEL', NULL, '2', 'S', 'none', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'B_gQ6kAQ_', '7c7cc7c5cc757', 1, 2524, 'nil', NULL, NULL, 575, NULL, 'tDdDdAQ'),
(2, 1, '_', NULL, 1, 5906, 'FALSE', NULL, 4095, NULL, 'TTTJJTTThTJJhJ', 'GGyGysGGssGsy'),
(3, 1, 'then', 'LPT1', 1, 218, NULL, 28, 1751, 896, NULL, '0');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, 2785, 1),
(2, 1, 1, 1, NULL, 532, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', '999999999999999999999999999999'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', 'J');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '2', '2222222222');
ROLLBACK;

-- ============================================================================
-- Generated database 020/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'COM1', '[de]', NULL, NULL, 'F', 'KzKKKzz'),
(2, 'Ruvv11uvvvuR1v1RR1RRu1', NULL, 7706, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 8999, 'False', 'F3FDz', 'n2sglgDs', NULL, 'o'),
(2, 'Downey Jr., Robert', NULL, 640, 'g', 'v6vIbb6b2WvVI6', NULL, 'EpEw5Aw', 'FzzzzzzF');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, 'O2ix', 'eee', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-QCCQ', NULL, 1, NULL, 1685, NULL, 3166, NULL, 276, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, '7f7fff', NULL, 'QRRXLL', '-QCCQ'),
(2, 1, 'i5''Uv5U''v''UU''vi5''i5UU', NULL, NULL, 'etjU', 'PPQQ-88--Q8-8PQ-Q--Q--8-8P8Q8-PP', 'false'),
(3, 2, 'O LEUPEL', NULL, '2', 'S', 'none', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'B_gQ6kAQ_', '7c7cc7c5cc757', 1, 2524, 'nil', NULL, NULL, 575, NULL, 'tDdDdAQ'),
(2, 1, '_', NULL, 1, 5906, 'FALSE', NULL, 4095, NULL, 'TTTJJTTThTJJhJ', 'GGyGysGGssGsy'),
(3, 1, 'then', 'LPT1', 1, 218, NULL, 28, 1751, 896, NULL, '0');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, 2785, 1),
(2, 1, 1, 1, NULL, 532, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', '999999999999999999999999999999'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', 'J');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'c 8', '2222222222');
ROLLBACK;

-- ============================================================================
-- Generated database 021/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'COM1', '[de]', NULL, NULL, 'F', 'KzKKKzz'),
(2, 'Ruvv11uvvvuR1v1RR1RRu1', NULL, 7706, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 8999, 'False', 'FzzzzzzF', 'n2sglgDs', NULL, 'o'),
(2, 'Downey Jr., Robert', NULL, 640, 'g', 'v6vIbb6b2WvVI6', NULL, 'EpEw5Aw', 'FzzzzzzF');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, 'O2ix', 'eee', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-QCCQ', NULL, 1, NULL, 1685, NULL, 3166, NULL, 276, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, '7f7fff', NULL, 'QRRXLL', 'xv'),
(2, 1, 'i5''Uv5U''v''UU''vi5''i5UU', NULL, NULL, 'etjU', 'PPQQ-88--Q8-8PQ-Q--Q--8-8P8Q8-PP', 'false'),
(3, 2, 'O LEUPEL', NULL, '2', 'S', 'none', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'B_gQ6kAQ_', '7c7cc7c5cc757', 1, 2524, 'nil', NULL, NULL, 575, NULL, 'tDdDdAQ'),
(2, 1, '_', NULL, 1, 5906, 'FALSE', NULL, 4095, NULL, 'TTTJJTTThTJJhJ', 'GGyGysGGssGsy'),
(3, 1, 'then', 'LPT1', 1, 218, NULL, 28, 1751, 896, NULL, '0');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, 2785, 1),
(2, 1, 1, 1, NULL, 532, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', '999999999999999999999999999999'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', 'J');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'c 8', '2222222222');
ROLLBACK;

-- ============================================================================
-- Generated database 022/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'COM1', '[de]', NULL, NULL, 'F', 'KzKKKzz'),
(2, 'Ruvv11uvvvuR1v1RR1RRu1', NULL, 7706, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 8999, 'False', 'F3FDz', 'n2sglgDs', NULL, 'o'),
(2, 'Downey Jr., Robert', NULL, 640, 'g', 'v6vIbb6b2WvVI6', NULL, 'EpEw5Aw', 'FzzzzzzF');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, 'O2ix', 'eee', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-QCCQ', NULL, 1, NULL, 1685, NULL, 3166, NULL, 276, NULL, NULL),
(2, '1', '1', 1, NULL, NULL, 'QRRXLL', 1, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '1', NULL, NULL, 'PPQQ-88--Q8-8PQ-Q--Q--8-8P8Q8-PP', 'false', NULL),
(2, 2, '1', '2', 'S', 'none', NULL, NULL),
(3, 2, '1', NULL, NULL, NULL, '1', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '1', '1', 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 2, '1', NULL, 1, NULL, '1', 1, NULL, 1, NULL, NULL),
(3, 2, '1', NULL, 1, NULL, '1', 28, 1751, 896, NULL, '0');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, 2785, 1),
(2, 1, 1, 1, NULL, 532, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', '999999999999999999999999999999'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', 'J');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'c 8', '2222222222');
ROLLBACK;

-- ============================================================================
-- Generated database 023/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '-Infinity', '[ru]', 193, '8hK', NULL, 'undefined'),
(2, 'czDT', '[de]', NULL, 'uj', '__dict__', 'Lc'),
(3, '9UdUKdY9dbU', NULL, 39, 'F', '0', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL),
(3, 'character-name-in-title', 'lplapCplaa0C');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 582, 'tz', NULL, NULL, NULL, ''),
(2, 'Downey Jr., Robert', 'PfTePPfj', NULL, NULL, NULL, NULL, NULL, 'f_R__fifi'),
(3, 'Downey Jr., Robert', NULL, NULL, 'xxxxxxxxxxxxxxxxx', 'Y6', NULL, 'else', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, 'l2D22Dl2llD2l2Dl', 'LPT1');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '75', NULL, 1, 2011, NULL, 'ppSS', NULL, 715, NULL, 'B____', NULL),
(2, 'f', NULL, 1, NULL, 250, NULL, NULL, 4806, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'H%m%m%O%O', 'eee', NULL, 'NaN', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''', 'bab', 1, NULL, NULL, NULL, 708, NULL, '__p_oI', NULL),
(2, 1, '''''_''1_wkYkmAw1wAY11''', NULL, 1, 16, '8O88OOO', 1006, NULL, 1035, NULL, NULL),
(3, 2, 'ekEEEkekeEkkeEEEeEEEkeEeEEk', NULL, 1, NULL, NULL, NULL, NULL, 1912, 'RVOeO1VV', 'OkI');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, NULL, 222, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'tUYUt11ttI'),
(2, 1, 2, 'USA', '00');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL),
(2, 2, 3, '4.0', '0'),
(3, 1, 2, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 1),
(2, 2, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'SSSS', NULL),
(2, 3, 2, '999', '5555555555'),
(3, 3, 1, 'Nypz', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 024/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '-Infinity', '[ru]', 193, '8hK', NULL, 'undefined'),
(2, 'czDT', '[de]', NULL, 'uj', '__dict__', 'Lc'),
(3, '9UdUKdY9dbU', NULL, 39, 'F', '0', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL),
(3, 'character-name-in-title', 'lplapCplaa0C');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 582, 'tz', NULL, NULL, NULL, ''),
(2, 'Downey Jr., Robert', 'PfTePPfj', NULL, NULL, NULL, NULL, NULL, 'f_R__fifi'),
(3, 'Downey Jr., Robert', NULL, NULL, 'xxxxxxxxxxxxxxxxx', 'Y6', NULL, 'RVOeO1VV', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, 'l2D22Dl2llD2l2Dl', 'LPT1');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '75', NULL, 1, 2011, NULL, 'ppSS', NULL, 715, NULL, 'B____', NULL),
(2, 'f', NULL, 1, NULL, 250, NULL, NULL, 4806, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'H%m%m%O%O', 'eee', NULL, 'NaN', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''', 'bab', 1, NULL, NULL, NULL, 708, NULL, '__p_oI', NULL),
(2, 1, '''''_''1_wkYkmAw1wAY11''', NULL, 1, 16, '8O88OOO', 1006, NULL, 1035, NULL, NULL),
(3, 2, 'ekEEEkekeEkkeEEEeEEEkeEeEEk', NULL, 1, NULL, NULL, NULL, NULL, 1912, 'RVOeO1VV', 'OkI');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, NULL, 222, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'tUYUt11ttI'),
(2, 1, 2, 'USA', '00');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL),
(2, 2, 3, '4.0', '0'),
(3, 1, 2, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 1),
(2, 2, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'SSSS', NULL),
(2, 3, 2, '999', '5555555555'),
(3, 3, 1, 'Nypz', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 025/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '-Infinity', '[ru]', 193, '8hK', NULL, 'undefined'),
(2, 'czDT', '[de]', NULL, 'uj', '__dict__', 'Lc'),
(3, '9UdUKdY9dbU', NULL, 39, 'F', '0', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL),
(3, 'character-name-in-title', 'lplapCplaa0C');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 582, NULL, NULL, NULL, NULL, NULL),
(2, 'Other Person', '', NULL, 'PfTePPfj', NULL, NULL, NULL, NULL),
(3, 'Other Person', 'f_R__fifi', NULL, NULL, NULL, 'xxxxxxxxxxxxxxxxx', 'Y6', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', NULL, 1, NULL, NULL, 'LPT1', NULL, NULL, NULL, '1', 'ppSS'),
(2, '1', '1', 1, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '1');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '1', NULL, 1, NULL, 'NaN', NULL, NULL, NULL, NULL, NULL),
(2, 2, 'bab', NULL, 1, NULL, NULL, 708, NULL, 1, NULL, NULL),
(3, 1, '''''_''1_wkYkmAw1wAY11''', NULL, 1, 16, '8O88OOO', 1006, NULL, 1035, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, 2, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL),
(2, 2, 2, 'USA', '1');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', NULL),
(2, 1, 2, '4.0', NULL),
(3, 1, 1, '8.0', 'tUYUt11ttI');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1),
(2, 2, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, '1', NULL),
(2, 2, 2, '1', NULL),
(3, 2, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 026/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '-Infinity', '[ru]', 193, '8hK', NULL, 'undefined'),
(2, 'czDT', '[de]', NULL, 'uj', '__dict__', 'Lc'),
(3, '9UdUKdY9dbU', NULL, 39, 'F', '0', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL),
(3, 'character-name-in-title', 'lplapCplaa0C');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 582, 'tz', NULL, NULL, NULL, ''),
(2, 'Downey Jr., Robert', 'PfTePPfj', NULL, NULL, NULL, NULL, NULL, 'f_R__fifi'),
(3, 'Other Person', NULL, NULL, 'xxxxxxxxxxxxxxxxx', 'Y6', NULL, 'else', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, 'l2D22Dl2llD2l2Dl', 'LPT1');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '75', NULL, 1, 2011, NULL, 'ppSS', NULL, 715, NULL, 'B____', NULL),
(2, 'f', NULL, 1, NULL, 250, NULL, NULL, 4806, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'H%m%m%O%O', 'eee', NULL, 'NaN', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''', 'bab', 1, NULL, NULL, NULL, 708, NULL, '__p_oI', NULL),
(2, 1, '''''_''1_wkYkmAw1wAY11''', NULL, 1, 16, '8O88OOO', 1006, NULL, 1035, NULL, NULL),
(3, 2, 'ekEEEkekeEkkeEEEeEEEkeEeEEk', NULL, 1, NULL, NULL, NULL, NULL, 1912, 'RVOeO1VV', 'OkI');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, NULL, 222, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'tUYUt11ttI'),
(2, 1, 2, 'USA', '00');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL),
(2, 2, 3, '4.0', '0'),
(3, 1, 2, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 1),
(2, 2, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'SSSS', NULL),
(2, 3, 2, '999', '5555555555'),
(3, 3, 1, 'Nypz', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 027/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'GAEGAG_zAGG', '[us]', 364, 'vlPv1', NULL, NULL),
(2, 'B', NULL, NULL, NULL, 'B', 'JZkZhooh4J4ko4sk4o');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'DTU66E4U'),
(2, 'character-name-in-title', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, 'iDw6Li', NULL, '1_t', NULL, NULL),
(2, 'Other Person', 'ii99', 158, NULL, 'YE EE1Y1mmuCu--CuE-m  mGGYQ-m', '1', '', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'XRRRRX', 108, 'LPT1', NULL, NULL),
(2, 'Voice Character', NULL, NULL, NULL, NULL, '%%'),
(3, 'Voice Character', NULL, NULL, NULL, NULL, '----');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'true', NULL, 1, NULL, NULL, NULL, NULL, 1150, 7851, 'Mzzzzz', 'fkJHWWkH'),
(2, 'Infinity', NULL, 1, 2005, NULL, NULL, 87, NULL, 3022, NULL, NULL),
(3, 'NaN', NULL, 1, NULL, NULL, 'p', NULL, NULL, 716, 'Um', '-');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '0', 'D3C3', 'gg55g5555jgjg5g55gggjj', 'COM1', 'Infinity', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', 'True', 1, 127, '4eg4g', 32, 4236, 457, 'Infinity', NULL),
(2, 2, '0', 'Zwss99', 1, NULL, 'else', 4, 6, NULL, 'LPT1', NULL),
(3, 3, 'XXXXXXXX', '', 1, 1390, NULL, NULL, 1009, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, NULL, NULL, 4, 1),
(2, 2, 2, NULL, '(uncredited)', 837, 1),
(3, 2, 3, NULL, NULL, 15, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, 'Qe_'),
(2, 3, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'USA', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', 'wwwU7w'),
(2, 3, 1, '8.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 3, 2),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '%%%%%%%%', '%UFUU-0'),
(2, 1, 1, 'O  COO', 'NIL');
ROLLBACK;

-- ============================================================================
-- Generated database 028/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'GAEGAG_zAGG', '[us]', 364, 'vlPv1', NULL, NULL),
(2, 'B', NULL, NULL, NULL, 'B', 'JZkZhooh4J4ko4sk4o');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'DTU66E4U'),
(2, 'character-name-in-title', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, 'iDw6Li', NULL, '1_t', NULL, NULL),
(2, 'Other Person', 'ii99', 158, NULL, 'YE EE1Y1mmuCu--CuE-m  mGGYQ-m', '1', '', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'XRRRRX', 108, 'LPT1', NULL, NULL),
(2, 'Voice Character', NULL, NULL, NULL, NULL, '%%'),
(3, 'Voice Character', NULL, NULL, NULL, NULL, '----');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'true', NULL, 1, NULL, NULL, NULL, NULL, 1150, 7851, 'Mzzzzz', 'fkJHWWkH'),
(2, 'Infinity', NULL, 1, 2005, NULL, NULL, 87, NULL, 3022, NULL, NULL),
(3, 'NaN', NULL, 1, NULL, NULL, 'p', NULL, NULL, 716, 'Um', '-');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '0', 'D3C3', 'gg55g5555jgjg5g55gggjj', 'COM1', 'Infinity', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', NULL, 1, NULL, NULL, NULL, 127, 1, NULL, '1'),
(2, 2, '', NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, 'Zwss99'),
(3, 1, '1', NULL, 1, 4, '1', 1, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, 1, '(voice)', NULL, 1),
(2, 2, 2, NULL, NULL, NULL, 1),
(3, 2, 2, NULL, '(voice)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, NULL),
(2, 2, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'USA', '1');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 2, 1, '4.0', NULL),
(3, 2, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 2, 2),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1),
(2, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '1', NULL),
(2, 2, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 029/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'GAEGAG_zAGG', '[us]', 364, 'vlPv1', NULL, NULL),
(2, 'B', NULL, NULL, NULL, 'B', 'JZkZhooh4J4ko4sk4o');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'DTU66E4U'),
(2, 'character-name-in-title', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, 'iDw6Li', NULL, '1_t', NULL, NULL),
(2, 'Other Person', 'ii99', 158, NULL, 'YE EE1Y1mmuCu--CuE-m  mGGYQ-m', '1', '', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'XRRRRX', 108, 'LPT1', NULL, NULL),
(2, 'Voice Character', NULL, NULL, NULL, NULL, '%%'),
(3, 'Voice Character', NULL, NULL, NULL, NULL, '----');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'true', NULL, 1, NULL, NULL, NULL, NULL, 1150, 7851, 'Mzzzzz', 'fkJHWWkH'),
(2, 'Infinity', NULL, 1, 2005, NULL, NULL, 87, NULL, 3022, NULL, NULL),
(3, 'NaN', NULL, 1, NULL, NULL, 'p', NULL, NULL, 1, 'Um', '-');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '0', 'D3C3', 'gg55g5555jgjg5g55gggjj', 'COM1', 'Infinity', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', 'True', 1, 127, '4eg4g', 32, 4236, 457, 'Infinity', NULL),
(2, 2, '0', 'Zwss99', 1, NULL, 'else', 4, 6, NULL, 'LPT1', NULL),
(3, 3, 'XXXXXXXX', '', 1, 1390, NULL, NULL, 1009, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, NULL, NULL, 4, 1),
(2, 2, 2, NULL, '(uncredited)', 837, 1),
(3, 2, 3, NULL, NULL, 15, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, 'Qe_'),
(2, 3, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'USA', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', 'wwwU7w'),
(2, 3, 1, '8.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 3, 2),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '%%%%%%%%', '%UFUU-0'),
(2, 1, 1, 'O  COO', 'NIL');
ROLLBACK;

-- ============================================================================
-- Generated database 030/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'GAEGAG_zAGG', '[us]', 364, 'vlPv1', NULL, NULL),
(2, 'B', NULL, NULL, NULL, 'B', 'JZkZhooh4J4ko4sk4o');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'DTU66E4U'),
(2, 'character-name-in-title', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, 'iDw6Li', NULL, '1_t', NULL, NULL),
(2, 'Other Person', 'ii99', 158, NULL, 'YE EE1Y1mmuCu--CuE-m  mGGYQ-m', '1', '', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'XRRRRX', 108, 'LPT1', NULL, NULL),
(2, 'Voice Character', NULL, NULL, NULL, NULL, '%%'),
(3, 'Voice Character', NULL, NULL, NULL, NULL, '----');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'true', NULL, 1, NULL, NULL, NULL, NULL, 1150, 7851, 'Mzzzzz', 'fkJHWWkH'),
(2, 'Infinity', NULL, 1, 2005, NULL, NULL, 87, NULL, 3022, NULL, NULL),
(3, 'NaN', NULL, 1, NULL, NULL, 'p', NULL, NULL, 716, 'Um', '-');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '0', 'D3C3', 'gg55g5555jgjg5g55gggjj', 'COM1', 'Infinity', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', 'True', 1, 127, '4eg4g', 32, 4236, 457, 'Infinity', NULL),
(2, 2, '0', 'Zwss99', 1, NULL, 'else', 4, 6, NULL, 'LPT1', NULL),
(3, 3, 'XXXXXXXX', '', 1, 1390, NULL, NULL, 1009, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, NULL, NULL, 4, 1),
(2, 2, 2, NULL, '(uncredited)', 837, 1),
(3, 2, 3, NULL, NULL, 15, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, 'Qe_'),
(2, 3, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 3, 1, 'USA', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', 'wwwU7w'),
(2, 3, 1, '4.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 3, 2),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '%%%%%%%%', '%UFUU-0'),
(2, 1, 1, 'O  COO', 'NIL');
ROLLBACK;

-- ============================================================================
-- Generated database 031/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'GAEGAG_zAGG', '[us]', 364, 'vlPv1', NULL, NULL),
(2, 'B', NULL, NULL, NULL, 'B', 'JZkZhooh4J4ko4sk4o');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'DTU66E4U'),
(2, 'character-name-in-title', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, 'iDw6Li', NULL, '1_t', NULL, NULL),
(2, 'Other Person', 'ii99', 158, NULL, 'YE EE1Y1mmuCu--CuE-m  mGGYQ-m', '1', '', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'XRRRRX', 108, 'LPT1', NULL, NULL),
(2, 'Voice Character', NULL, NULL, NULL, NULL, '%%'),
(3, 'Voice Character', NULL, NULL, NULL, NULL, '----');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'true', NULL, 1, NULL, NULL, NULL, NULL, 1150, 7851, 'Mzzzzz', 'fkJHWWkH'),
(2, 'Infinity', NULL, 1, 2005, NULL, NULL, 87, NULL, 3022, NULL, NULL),
(3, 'NaN', NULL, 1, NULL, NULL, 'p', NULL, NULL, 716, 'Um', '-');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '0', 'D3C3', 'gg55g5555jgjg5g55gggjj', 'COM1', 'Infinity', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', 'True', 1, 127, '4eg4g', 32, 4236, 457, 'Infinity', NULL),
(2, 2, '0', 'Zwss99', 1, NULL, 'else', 4, 6, NULL, 'LPT1', NULL),
(3, 3, 'XXXXXXXX', '', 1, 1390, NULL, NULL, 1009, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, NULL, NULL, 4, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, NULL),
(2, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL),
(2, 2, 1, '4.0', 'Qe_'),
(3, 3, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 1),
(2, 1, 2),
(3, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 2, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '1', NULL),
(2, 1, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 032/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'QQf', '[us]', NULL, '', 'NaN', NULL),
(2, '0', '[ru]', NULL, '4', NULL, NULL),
(3, '', '[ru]', 118, NULL, '', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'YYWWY'),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 479, 'c''''xLs''s7', '', NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 228, 'l4ckktWHkSYlWc', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '%mT%TNmT%%Bm%B%mN', 'MMMMM', 1, 2007, 1192, 'NULL', 3695, 149, 382, NULL, ''),
(2, 'WffSfWSzzzSS', NULL, 1, NULL, NULL, '', NULL, NULL, NULL, NULL, 'ggg');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, NULL, 'Q', NULL),
(2, 1, '', '00', NULL, NULL, '2', '-Y--');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'Vu-lq', '9', 1, NULL, NULL, 491, 2109, 17, NULL, 'NIL'),
(2, 1, ' 1gV', 'angjka5', 1, NULL, 'False', NULL, NULL, 232, NULL, 'i');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 1, NULL, 709, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, 'qoIfqVtftqeVAEeeVVEqftttm'),
(2, 1, 3, 1, NULL),
(3, 2, 3, 1, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'null');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', 'DDQQQDQQDDDD');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, '', '999999999999999999999999999999'),
(2, 1, 3, 'False', 'undefined'),
(3, 1, 1, 'q', '1e100');
ROLLBACK;

-- ============================================================================
-- Generated database 033/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'QQf', '[us]', NULL, '', 'NaN', NULL),
(2, '0', '[ru]', NULL, '4', NULL, NULL),
(3, '', '[ru]', 118, NULL, '', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'YYWWY'),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 479, 'c''''xLs''s7', '', NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 228, 'l4ckktWHkSYlWc', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '%mT%TNmT%%Bm%B%mN', 'MMMMM', 1, 2007, 1192, 'NULL', 3695, 149, 382, NULL, ''),
(2, 'WffSfWSzzzSS', NULL, 1, NULL, NULL, '', NULL, NULL, NULL, NULL, 'ggg');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, NULL, 'Q', NULL),
(2, 1, '', '00', NULL, NULL, '2', '-Y--');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'Vu-lq', '9', 1, NULL, NULL, 491, 2109, 17, NULL, 'NIL'),
(2, 1, ' 1gV', 'angjka5', 1, NULL, 'False', NULL, NULL, 232, NULL, 'i');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 1, NULL, 709, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, 'qoIfqVtftqeVAEeeVVEqftttm'),
(2, 1, 3, 1, NULL),
(3, 2, 3, 1, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'null');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', 'DDQQQDQQDDDD');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, '', '999999999999999999999999999999'),
(2, 1, 3, 'False', 'c''''xLs''s7'),
(3, 1, 1, 'q', '1e100');
ROLLBACK;

-- ============================================================================
-- Generated database 034/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'QQf', '[us]', NULL, '', 'NaN', NULL),
(2, '0', '[ru]', NULL, '4', NULL, NULL),
(3, '', '[ru]', 118, NULL, '', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'YYWWY'),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 479, 'c''''xLs''s7', '', NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 228, 'l4ckktWHkSYlWc', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '%mT%TNmT%%Bm%B%mN', 'MMMMM', 1, 2007, 1192, 'NULL', 3695, 149, 382, NULL, ''),
(2, 'WffSfWSzzzSS', NULL, 1, NULL, NULL, '', NULL, NULL, NULL, NULL, 'ggg');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, NULL, 'Q', NULL),
(2, 1, '', '00', NULL, NULL, '2', '-Y--');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'Vu-lq', '9', 1, NULL, NULL, 491, 2109, 17, NULL, 'NIL'),
(2, 1, ' 1gV', 'angjka5', 1, NULL, 'False', NULL, NULL, 232, NULL, 'i');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 1, NULL, 709, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, 'qoIfqVtftqeVAEeeVVEqftttm'),
(2, 1, 3, 1, NULL),
(3, 2, 3, 1, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'null');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', 'DDQQQDQQDDDD');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, '', '999999999999999999999999999999'),
(2, 1, 3, 'False', 'undefined'),
(3, 1, 2, 'q', '1e100');
ROLLBACK;

-- ============================================================================
-- Generated database 035/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'QQf', '[ru]', NULL, '', 'NaN', NULL),
(2, '0', '[ru]', NULL, '4', NULL, NULL),
(3, '', '[ru]', 118, NULL, '', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'YYWWY'),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 479, 'c''''xLs''s7', '', NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 228, 'l4ckktWHkSYlWc', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '%mT%TNmT%%Bm%B%mN', 'MMMMM', 1, 2007, 1192, 'NULL', 3695, 149, 382, NULL, ''),
(2, 'WffSfWSzzzSS', NULL, 1, NULL, NULL, '', NULL, NULL, NULL, NULL, 'ggg');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, NULL, 'Q', NULL),
(2, 1, '', '00', NULL, NULL, '2', '-Y--');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'Vu-lq', '9', 1, NULL, NULL, 491, 2109, 17, NULL, 'NIL'),
(2, 1, ' 1gV', 'angjka5', 1, NULL, 'False', NULL, NULL, 232, NULL, 'i');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 1, NULL, 709, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, 'qoIfqVtftqeVAEeeVVEqftttm'),
(2, 1, 3, 1, NULL),
(3, 2, 3, 1, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'null');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', 'DDQQQDQQDDDD');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, '', '999999999999999999999999999999'),
(2, 1, 3, 'False', 'undefined'),
(3, 1, 1, 'q', '1e100');
ROLLBACK;

-- ============================================================================
-- Generated database 036/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'QQf', '[us]', NULL, '', 'DDQQQDQQDDDD', NULL),
(2, '0', '[ru]', NULL, '4', NULL, NULL),
(3, '', '[ru]', 118, NULL, '', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'YYWWY'),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 479, 'c''''xLs''s7', '', NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 228, 'l4ckktWHkSYlWc', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '%mT%TNmT%%Bm%B%mN', 'MMMMM', 1, 2007, 1192, 'NULL', 3695, 149, 382, NULL, ''),
(2, 'WffSfWSzzzSS', NULL, 1, NULL, NULL, '', NULL, NULL, NULL, NULL, 'ggg');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, NULL, 'Q', NULL),
(2, 1, '', '00', NULL, NULL, '2', '-Y--');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'Vu-lq', '9', 1, NULL, NULL, 491, 2109, 17, NULL, 'NIL'),
(2, 1, ' 1gV', 'angjka5', 1, NULL, 'False', NULL, NULL, 232, NULL, 'i');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 1, NULL, 709, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 1, 'qoIfqVtftqeVAEeeVVEqftttm'),
(2, 1, 3, 1, NULL),
(3, 2, 3, 1, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'null');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', 'DDQQQDQQDDDD');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, '', '999999999999999999999999999999'),
(2, 1, 3, 'False', 'undefined'),
(3, 1, 1, 'q', '1e100');
ROLLBACK;

-- ============================================================================
-- Generated database 037/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'undefined', '[ru]', 283, NULL, 'QFF%vggegFeFc%ve', NULL),
(2, '999999999999999999999999999999', '[de]', 11, NULL, 'EEVVEVE', 'H');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', NULL, NULL, 'H22LHHLC', 'd''LLLgd2d''2L''Ku''g''''', 'NNYNYfYfYNfffffYYNNffYYY', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '__dict__', NULL, 'TRUE');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 181, 'TTTTTTTTTTTTTTT', NULL, NULL),
(2, 'Voice Character', 'H', 171, NULL, NULL, NULL),
(3, 'Other Character', NULL, NULL, NULL, 'wS', 'II');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__dict__', NULL, 1, 2005, NULL, NULL, NULL, 428, 4975, 'TRUE', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'jFjjjj', NULL, NULL, 'E', '999999999999999999999999999999', 'Scunthorpe'),
(2, 1, '44   l', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'None', NULL, 2, 486, NULL, NULL, 7477, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, 5952, 1),
(2, 2, 1, 1, '(voice)', 2900, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 3, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 2, 'NULL'),
(2, 1, 1, 1, NULL),
(3, 1, 2, 1, 'XiDXciiiiX');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '5', NULL),
(2, 2, 1, 'else', '2JJ-2Jjpp2-Jj2''2-Jp-j22-J-p'),
(3, 2, 1, '', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 038/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'undefined', '[ru]', 283, NULL, 'QFF%vggegFeFc%ve', NULL),
(2, '999999999999999999999999999999', '[de]', 11, NULL, 'EEVVEVE', 'H');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', NULL, NULL, 'H22LHHLC', 'd''LLLgd2d''2L''Ku''g''''', 'NNYNYfYfYNfffffYYNNffYYY', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '__dict__', NULL, 'TRUE');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 181, 'TTTTTTTTTTTTTTT', NULL, NULL),
(2, 'Voice Character', 'H', 171, NULL, NULL, NULL),
(3, 'Other Character', NULL, NULL, NULL, 'wS', 'II');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__dict__', NULL, 1, 2005, NULL, NULL, NULL, 428, 4975, 'TRUE', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'jFjjjj', NULL, NULL, 'E', '999999999999999999999999999999', 'Scunthorpe'),
(2, 1, '44   l', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'None', NULL, 2, 486, NULL, NULL, 7477, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, 5952, 1),
(2, 2, 1, 1, '(voice)', 2900, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 3, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 2, 'NULL'),
(2, 1, 1, 1, NULL),
(3, 1, 2, 1, 'XiDXciiiiX');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'else', NULL),
(2, 2, 1, 'else', '2JJ-2Jjpp2-Jj2''2-Jp-j22-J-p'),
(3, 2, 1, '', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 039/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'undefined', '[ru]', 283, NULL, 'QFF%vggegFeFc%ve', NULL),
(2, '999999999999999999999999999999', '[de]', 11, NULL, 'EEVVEVE', 'H');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', NULL, NULL, 'H22LHHLC', 'd''LLLgd2d''2L''Ku''g''''', 'NNYNYfYfYNfffffYYNNffYYY', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '__dict__', NULL, 'TRUE');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 181, 'TTTTTTTTTTTTTTT', NULL, NULL),
(2, 'Voice Character', 'H', 171, NULL, NULL, NULL),
(3, 'Other Character', NULL, NULL, NULL, 'wS', 'II');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__dict__', NULL, 1, 2005, NULL, NULL, NULL, 428, 4975, 'II', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'jFjjjj', NULL, NULL, 'E', '999999999999999999999999999999', 'Scunthorpe'),
(2, 1, '44   l', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'None', NULL, 2, 486, NULL, NULL, 7477, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, 5952, 1),
(2, 2, 1, 1, '(voice)', 2900, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 3, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 2, 'NULL'),
(2, 1, 1, 1, NULL),
(3, 1, 2, 1, 'XiDXciiiiX');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '5', NULL),
(2, 2, 1, 'else', '2JJ-2Jjpp2-Jj2''2-Jp-j22-J-p'),
(3, 2, 1, '', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 040/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'undefined', '[ru]', 283, NULL, 'QFF%vggegFeFc%ve', NULL),
(2, '999999999999999999999999999999', '[de]', 11, NULL, 'EEVVEVE', 'H');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', NULL, NULL, 'H22LHHLC', 'd''LLLgd2d''2L''Ku''g''''', 'NNYNYfYfYNfffffYYNNffYYY', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '__dict__', NULL, 'TRUE');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 181, 'TTTTTTTTTTTTTTT', NULL, NULL),
(2, 'Voice Character', 'H', 171, NULL, NULL, NULL),
(3, 'Other Character', NULL, NULL, NULL, 'wS', 'II');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__dict__', NULL, 1, 2005, NULL, NULL, NULL, 428, 4975, 'TRUE', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'jFjjjj', NULL, NULL, 'E', '999999999999999999999999999999', 'Scunthorpe'),
(2, 1, '44   l', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'None', NULL, 2, 486, NULL, NULL, 7477, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, 5952, 1),
(2, 2, 1, 1, '(voice)', 2900, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 3, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 2, 'NULL'),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', 'XiDXciiiiX');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '1', NULL),
(2, 1, 1, '1', NULL),
(3, 1, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 041/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'undefined', '[ru]', 283, NULL, 'QFF%vggegFeFc%ve', NULL),
(2, '999999999999999999999999999999', '[de]', 11, NULL, 'EEVVEVE', 'H');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '', NULL, NULL, 'H22LHHLC', 'd''LLLgd2d''2L''Ku''g''''', 'NNYNYfYfYNfffffYYNNffYYY', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '__dict__', '', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, '1', NULL, NULL),
(2, 'Other Character', NULL, NULL, 'H', '1', NULL),
(3, 'Other Character', NULL, NULL, NULL, NULL, 'wS');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', NULL, 2, NULL, NULL, NULL, 2005, NULL, NULL, NULL, '1');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, NULL, NULL, NULL, NULL),
(2, 2, '1', 'E', '999999999999999999999999999999', 'Scunthorpe', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '1', NULL, 2, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, '(voice)', NULL, 1),
(2, 2, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2),
(2, NULL, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 2, 1, NULL),
(3, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL),
(2, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '1', NULL),
(2, 2, 1, '1', NULL),
(3, 1, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 042/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LVKsLVs0sDLsKVs0', '[us]', 42, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, '5u5995Bu99555B99u955uBu', 'OO', NULL, NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, 'KUUY''Y', 'u', NULL, NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, 'nil', 'y', NULL, 't9t3293');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '', 257, '', NULL, ''),
(2, 'Other Character', NULL, NULL, 'kn''Anc6kUn66Ac''nk', 'INF', 'eL'),
(3, 'Voice Character', NULL, 2280, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 's', NULL, 1, NULL, 57, NULL, NULL, NULL, NULL, 'none', 'DIAIC'),
(2, 'none', NULL, 1, 2008, NULL, NULL, 584, 2085, NULL, 'XX', 'JbJzb8bzzz8b'),
(3, 'N6PN6PEgEPTEEP', '5uPh5', 1, NULL, 2671, NULL, NULL, NULL, NULL, 'H88hfiyJJyf8y7SHh7HhfHihSHy7h88H', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '_', 'nil', NULL, NULL, '1e100', 'ii4iii44iii44'),
(2, 3, 'mmmr', NULL, 'Ff6PDSDF6ScSPDFf6P', NULL, 'vV', 'ssQDNQQt');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 3, 'vUUUvUvUvvUvUUUUvUUU', NULL, 1, 1239, NULL, NULL, 13, 1042, NULL, NULL),
(2, 2, 'pKz', NULL, 1, 365, 'then', NULL, 605, 458, '', 'NIL');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 3, NULL, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, 2, 2, 2),
(3, 3, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 3, NULL),
(2, 2, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'Bulgaria', 'e');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL),
(2, 3, 1, '8.0', '999999999999999999999999999999'),
(3, 2, 1, '4.0', '5dO5L');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 3, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1),
(2, 3, 2, 2),
(3, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 3, 2, 'nil', '0');
ROLLBACK;

-- ============================================================================
-- Generated database 043/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LVKsLVs0sDLsKVs0', '[us]', 42, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, '5u5995Bu99555B99u955uBu', 'OO', NULL, NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, 'KUUY''Y', 'u', NULL, NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, 'nil', 'y', NULL, 't9t3293');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '', 257, '', NULL, ''),
(2, 'Other Character', NULL, NULL, 'kn''Anc6kUn66Ac''nk', 'INF', 'eL'),
(3, 'Voice Character', NULL, 2280, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 's', NULL, 1, NULL, 57, NULL, NULL, NULL, NULL, 'none', 'DIAIC'),
(2, 'none', NULL, 1, 2008, NULL, NULL, 584, 2085, NULL, 'XX', 'JbJzb8bzzz8b'),
(3, 'N6PN6PEgEPTEEP', '5uPh5', 1, NULL, 2671, NULL, NULL, NULL, NULL, 'H88hfiyJJyf8y7SHh7HhfHihSHy7h88H', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '_', 'nil', NULL, NULL, '1e100', 'ii4iii44iii44'),
(2, 3, 'mmmr', NULL, 'Ff6PDSDF6ScSPDFf6P', NULL, 'vV', 'ssQDNQQt');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 3, 'vUUUvUvUvvUvUUUUvUUU', NULL, 1, 1239, NULL, NULL, 13, 1042, NULL, NULL),
(2, 2, 'pKz', NULL, 1, 365, 'then', NULL, 605, 458, '', 'N6PN6PEgEPTEEP');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 3, NULL, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, 2, 2, 2),
(3, 3, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 3, NULL),
(2, 2, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'Bulgaria', 'e');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL),
(2, 3, 1, '8.0', '999999999999999999999999999999'),
(3, 2, 1, '4.0', '5dO5L');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 3, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1),
(2, 3, 2, 2),
(3, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 3, 2, 'nil', '0');
ROLLBACK;

-- ============================================================================
-- Generated database 044/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LVKsLVs0sDLsKVs0', '[us]', 42, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, '5u5995Bu99555B99u955uBu', 'OO', NULL, NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, 'KUUY''Y', 'u', NULL, NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, 'nil', 'y', NULL, 't9t3293');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actor'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '', 257, '', NULL, ''),
(2, 'Other Character', NULL, NULL, 'kn''Anc6kUn66Ac''nk', 'INF', 'eL'),
(3, 'Voice Character', NULL, 2280, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 's', NULL, 1, NULL, 57, NULL, NULL, NULL, NULL, 'none', 'JbJzb8bzzz8b'),
(2, 'none', NULL, 1, 2008, NULL, NULL, 584, 2085, NULL, 'XX', 'JbJzb8bzzz8b'),
(3, 'N6PN6PEgEPTEEP', '5uPh5', 1, NULL, 2671, NULL, NULL, NULL, NULL, 'H88hfiyJJyf8y7SHh7HhfHihSHy7h88H', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '_', 'nil', NULL, NULL, '1e100', 'ii4iii44iii44'),
(2, 3, 'mmmr', NULL, 'Ff6PDSDF6ScSPDFf6P', NULL, 'vV', 'ssQDNQQt');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 3, 'vUUUvUvUvvUvUUUUvUUU', NULL, 1, 1239, NULL, NULL, 13, 1042, NULL, NULL),
(2, 2, 'pKz', NULL, 1, 365, 'then', NULL, 605, 458, '', 'NIL');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 3, NULL, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, 2, 2, 2),
(3, 3, 2, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 3, NULL),
(2, 2, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'Bulgaria', 'e');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL),
(2, 3, 1, '8.0', '999999999999999999999999999999'),
(3, 2, 1, '4.0', '5dO5L');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 3, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1),
(2, 3, 2, 2),
(3, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 3, 2, 'nil', '0');
ROLLBACK;

-- ============================================================================
-- Generated database 045/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LVKsLVs0sDLsKVs0', '[us]', 42, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, NULL, '5u5995Bu99555B99u955uBu', 'OO', NULL, NULL),
(2, 'Other Person', NULL, NULL, NULL, 'KUUY''Y', 'u', NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, NULL, 'nil', 'y', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, NULL, '', '1'),
(2, 'Other Character', NULL, 1, NULL, NULL, NULL),
(3, 'Other Character', NULL, NULL, 'INF', 'eL', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', '1', 1, NULL, NULL, NULL, NULL, NULL, 57, NULL, NULL),
(2, '1', NULL, 1, NULL, NULL, 'DIAIC', NULL, NULL, NULL, '1', NULL),
(3, '1', NULL, 1, NULL, NULL, 'XX', 1, NULL, NULL, '5uPh5', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, NULL, NULL, NULL, NULL),
(2, 2, 'H88hfiyJJyf8y7SHh7HhfHihSHy7h88H', NULL, NULL, NULL, NULL, 'nil');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '1', NULL, 1, NULL, 'ii4iii44iii44', NULL, NULL, NULL, NULL, 'Ff6PDSDF6ScSPDFf6P'),
(2, 2, '', NULL, 1, 1, NULL, NULL, NULL, NULL, NULL, '1');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, '(voice)', NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, NULL, 1, 2),
(3, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 2, NULL),
(2, 2, 1, 2, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', NULL),
(2, 2, 2, '4.0', '1'),
(3, 2, 2, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1),
(2, 2, 2, 2),
(3, 1, 2, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 046/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LPT1', '[de]', NULL, NULL, NULL, 'umu7m');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 2540, NULL, 'Scunthorpe', 'flAlleefAAFlGfAlAAG', 'UN5N3lq55U5N3q5UqN', NULL),
(2, 'Downey Jr., Robert', 'yyyyJyyyycJYycJcc', NULL, 'SSSSS', 'NNvUP', '-Uvvvv', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, 'PkD', NULL, '__dict__', NULL, 'X');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1988, NULL, NULL, 'eeeeeeeeeee'),
(2, 'Voice Character', NULL, NULL, NULL, 'false', NULL),
(3, 'Other Character', '', NULL, 'yMyBB', 'w', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6', 'Scunthorpe', 2, NULL, 1023, NULL, 63, 31, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'bbbbbbbbb', 'True', 'l8YcLLY8LXrLGjxYLXLj', 'HhI VPhhh IVhuPu IhuhH9 VP9uIu', NULL, NULL),
(2, 1, '----', NULL, NULL, 'if', NULL, 'L'),
(3, 1, 'wNZ', NULL, NULL, 'ut', 'r', '_oo_ooooo_o_o_');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''M''U', 'D', 1, NULL, 'uNJ', NULL, NULL, 705, NULL, NULL),
(2, 1, 'Inf', NULL, 1, NULL, NULL, 429, NULL, NULL, 'Inf', NULL),
(3, 1, 'q9mm', NULL, 1, NULL, NULL, 8192, NULL, NULL, NULL, 'undefined');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, NULL, NULL, 1),
(2, 2, 1, NULL, '(voice) (uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 1),
(2, 1, 1, 3),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 2, 'WmuWWWmmmWu');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'USA', ' n nnn'),
(3, 1, 1, 'USA', 'klkllk');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '8.0', 'undefined');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '11jHj1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 047/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LPT1', '[de]', NULL, NULL, NULL, 'umu7m');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 2540, NULL, 'Scunthorpe', 'flAlleefAAFlGfAlAAG', 'UN5N3lq55U5N3q5UqN', NULL),
(2, 'Downey Jr., Robert', 'yyyyJyyyycJYycJcc', NULL, 'SSSSS', 'NNvUP', '-Uvvvv', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, 'PkD', NULL, '__dict__', NULL, 'X');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1988, NULL, NULL, 'eeeeeeeeeee'),
(2, 'Voice Character', NULL, NULL, NULL, 'false', NULL),
(3, 'Other Character', '', NULL, 'yMyBB', 'w', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6', 'Scunthorpe', 2, NULL, 1023, NULL, 63, 31, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'bbbbbbbbb', 'True', 'l8YcLLY8LXrLGjxYLXLj', 'HhI VPhhh IVhuPu IhuhH9 VP9uIu', NULL, NULL),
(2, 1, '----', NULL, NULL, 'umu7m', NULL, 'L'),
(3, 1, 'wNZ', NULL, NULL, 'ut', 'r', '_oo_ooooo_o_o_');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''M''U', 'D', 1, NULL, 'uNJ', NULL, NULL, 705, NULL, NULL),
(2, 1, 'Inf', NULL, 1, NULL, NULL, 429, NULL, NULL, 'Inf', NULL),
(3, 1, 'q9mm', NULL, 1, NULL, NULL, 8192, NULL, NULL, NULL, 'undefined');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, NULL, NULL, 1),
(2, 2, 1, NULL, '(voice) (uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 1),
(2, 1, 1, 3),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 2, 'WmuWWWmmmWu');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'USA', ' n nnn'),
(3, 1, 1, 'USA', 'klkllk');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '8.0', 'undefined');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '11jHj1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 048/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LPT1', '[de]', NULL, NULL, NULL, 'umu7m');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 2540, NULL, 'Scunthorpe', 'flAlleefAAFlGfAlAAG', 'UN5N3lq55U5N3q5UqN', NULL),
(2, 'Downey Jr., Robert', 'yyyyJyyyycJYycJcc', NULL, 'SSSSS', 'NNvUP', '-Uvvvv', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, 'PkD', NULL, '__dict__', NULL, 'X');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1988, NULL, NULL, 'eeeeeeeeeee'),
(2, 'Voice Character', NULL, NULL, NULL, 'false', NULL),
(3, 'Other Character', '', NULL, 'yMyBB', 'w', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6', 'Scunthorpe', 2, NULL, 1023, NULL, 63, 31, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'bbbbbbbbb', 'True', 'l8YcLLY8LXrLGjxYLXLj', 'HhI VPhhh IVhuPu IhuhH9 VP9uIu', NULL, NULL),
(2, 1, '----', NULL, NULL, 'if', NULL, 'L'),
(3, 1, 'wNZ', NULL, NULL, 'ut', 'r', '_oo_ooooo_o_o_');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''M''U', 'D', 1, NULL, 'uNJ', NULL, 1, NULL, NULL, NULL),
(2, 1, 'Inf', NULL, 1, NULL, NULL, 429, NULL, NULL, 'Inf', NULL),
(3, 1, 'q9mm', NULL, 1, NULL, NULL, 8192, NULL, NULL, NULL, 'undefined');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, NULL, NULL, 1),
(2, 2, 1, NULL, '(voice) (uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 1),
(2, 1, 1, 3),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 2, 'WmuWWWmmmWu');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'USA', ' n nnn'),
(3, 1, 1, 'USA', 'klkllk');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '8.0', 'undefined');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '11jHj1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 049/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'LPT1', '[de]', NULL, NULL, NULL, 'umu7m');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 2540, NULL, 'Scunthorpe', 'flAlleefAAFlGfAlAAG', 'UN5N3lq55U5N3q5UqN', NULL),
(2, 'Downey Jr., Robert', 'yyyyJyyyycJYycJcc', NULL, 'SSSSS', 'NNvUP', '-Uvvvv', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, 'PkD', NULL, '__dict__', NULL, 'X');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1988, NULL, NULL, 'eeeeeeeeeee'),
(2, 'Voice Character', NULL, NULL, NULL, 'false', NULL),
(3, 'Other Character', '', NULL, 'yMyBB', 'w', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '6', 'Scunthorpe', 2, NULL, 1023, NULL, 63, 31, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'bbbbbbbbb', 'True', 'l8YcLLY8LXrLGjxYLXLj', 'HhI VPhhh IVhuPu IhuhH9 VP9uIu', NULL, NULL),
(2, 1, '----', NULL, NULL, 'if', NULL, 'L'),
(3, 1, 'wNZ', NULL, NULL, 'bbbbbbbbb', 'r', '_oo_ooooo_o_o_');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '''M''U', 'D', 1, NULL, 'uNJ', NULL, NULL, 705, NULL, NULL),
(2, 1, 'Inf', NULL, 1, NULL, NULL, 429, NULL, NULL, 'Inf', NULL),
(3, 1, 'q9mm', NULL, 1, NULL, NULL, 8192, NULL, NULL, NULL, 'undefined');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, NULL, NULL, 1),
(2, 2, 1, NULL, '(voice) (uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 1),
(2, 1, 1, 3),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 2, 'WmuWWWmmmWu');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'USA', ' n nnn'),
(3, 1, 1, 'USA', 'klkllk');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '8.0', 'undefined');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2),
(2, 1, 1, 1),
(3, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '11jHj1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 050/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'CC0', '[ru]', 399, NULL, NULL, 'false'),
(2, '__dict__', NULL, 386, 'NULL', 'j', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'if'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'kK', NULL, NULL, 'h6BhhB', NULL, 'NaN', NULL),
(2, 'Downey Jr., Robert', NULL, 255, 'H', NULL, NULL, 'rrhhnrrHrr', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '7', 4, NULL, 'false', NULL),
(2, 'Other Character', NULL, NULL, 'V', NULL, 'sGJ-');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'NXXXNfX', NULL, 1, NULL, NULL, NULL, 256, NULL, 5, 'qqUUTRTqUqttT', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'xaeQe_8', NULL, '4II', NULL, 'DDDDDDDDDDDDDDDDDD', NULL),
(2, 1, 'LPT1', 'vs2s2s2s222s', NULL, 'True', '-Infinity', 'then'),
(3, 2, '-yyy-yz', NULL, '00', NULL, 'None', 'if');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', NULL, 2, 290, 'GNGY', NULL, NULL, 127, 'Y5''w', '00UfZZIuD0Zh0'),
(2, 1, 'n', 'c', 1, NULL, NULL, NULL, 1023, 2047, 'NaN', 'Th6ccZ');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 2, 2),
(3, NULL, 2, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', 'TRUE'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'Bulgaria', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '8.0', ''),
(2, 1, 2, '8.0', 'nil');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 2),
(3, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, 'BB', 'then');
ROLLBACK;

-- ============================================================================
-- Generated database 051/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'CC0', '[ru]', 399, NULL, NULL, 'false'),
(2, '__dict__', NULL, 386, 'NULL', 'j', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'if'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'kK', NULL, NULL, 'h6BhhB', NULL, 'NaN', NULL),
(2, 'Downey Jr., Robert', NULL, 255, 'H', NULL, NULL, 'rrhhnrrHrr', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '7', 4, NULL, 'false', NULL),
(2, 'Other Character', NULL, NULL, 'V', NULL, 'sGJ-');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'NXXXNfX', NULL, 1, NULL, NULL, NULL, 256, NULL, 5, 'qqUUTRTqUqttT', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'vs2s2s2s222s', NULL, '4II', NULL, 'DDDDDDDDDDDDDDDDDD', NULL),
(2, 1, 'LPT1', 'vs2s2s2s222s', NULL, 'True', '-Infinity', 'then'),
(3, 2, '-yyy-yz', NULL, '00', NULL, 'None', 'if');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', NULL, 2, 290, 'GNGY', NULL, NULL, 127, 'Y5''w', '00UfZZIuD0Zh0'),
(2, 1, 'n', 'c', 1, NULL, NULL, NULL, 1023, 2047, 'NaN', 'Th6ccZ');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 2, 2),
(3, NULL, 2, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', 'TRUE'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'Bulgaria', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '8.0', ''),
(2, 1, 2, '8.0', 'nil');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 2),
(3, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, 'BB', 'then');
ROLLBACK;

-- ============================================================================
-- Generated database 052/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'CC0', '[ru]', 399, NULL, NULL, 'false'),
(2, '__dict__', NULL, 386, 'NULL', 'j', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'if'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'kK', NULL, NULL, 'h6BhhB', NULL, 'NaN', NULL),
(2, 'Downey Jr., Robert', NULL, 255, 'H', NULL, NULL, 'rrhhnrrHrr', '1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 4, NULL, 'false', NULL),
(2, 'Other Character', NULL, NULL, 'V', NULL, 'sGJ-');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'NXXXNfX', NULL, 1, NULL, NULL, NULL, 256, NULL, 5, 'qqUUTRTqUqttT', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'xaeQe_8', NULL, '4II', NULL, 'DDDDDDDDDDDDDDDDDD', NULL),
(2, 1, 'LPT1', 'vs2s2s2s222s', NULL, 'True', '-Infinity', 'then'),
(3, 2, '-yyy-yz', NULL, '00', NULL, 'None', 'if');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', NULL, 2, 290, 'GNGY', NULL, NULL, 127, 'Y5''w', '00UfZZIuD0Zh0'),
(2, 1, 'n', 'c', 1, NULL, NULL, NULL, 1023, 2047, 'NaN', 'Th6ccZ');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 3, 3),
(2, 1, 2, 2),
(3, NULL, 2, 3);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', 'TRUE'),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'Bulgaria', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '8.0', ''),
(2, 1, 2, '8.0', 'nil');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 2),
(3, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, 'BB', 'then');
ROLLBACK;

-- ============================================================================
-- Generated database 053/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'CC0', '[ru]', 399, NULL, NULL, 'false'),
(2, '__dict__', NULL, 386, 'NULL', 'j', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'if'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'kK', NULL, NULL, 'h6BhhB', NULL, 'NaN', NULL),
(2, 'Downey Jr., Robert', NULL, 255, 'H', NULL, NULL, 'rrhhnrrHrr', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', '7', NULL, NULL, NULL, 'false'),
(2, 'Other Character', NULL, NULL, NULL, 'V', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', NULL, 2, NULL, NULL, NULL, NULL, NULL, NULL, '1', '1');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '1', NULL, NULL, NULL, NULL, '4II'),
(2, 2, '', NULL, NULL, NULL, NULL, NULL),
(3, 2, '', NULL, NULL, NULL, 'True', '-Infinity');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'then', NULL, 2, NULL, NULL, 1, NULL, NULL, 'None', 'if'),
(2, 1, '', NULL, 2, 290, 'GNGY', NULL, NULL, 127, 'Y5''w', '00UfZZIuD0Zh0');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, 1, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, NULL, 2, 2),
(3, 1, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 1, NULL),
(2, 1, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', NULL),
(2, 1, 2, 'USA', NULL),
(3, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', NULL),
(2, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 2),
(3, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 054/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', NULL, NULL, NULL, NULL, 'none'),
(2, 'jy', '[de]', 277, NULL, NULL, 'if'),
(3, '00', NULL, 585, NULL, 'bDK', 'none');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '999'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'IKK--KKK-KK', NULL, NULL, 'W7TWWB', NULL, 'if', 'BM'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, '', NULL, NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 33, '44r4m6h', NULL, NULL),
(2, 'Voice Character', NULL, 3418, NULL, 'LPT1', '99');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'sp%psOsO%', NULL, 1, 2006, 6, '__dict__', 462, NULL, 11, NULL, NULL),
(2, '%cc%%', NULL, 1, 2005, NULL, NULL, 125, 63, NULL, NULL, NULL),
(3, '1v28080Eu', NULL, 1, 2005, 287, NULL, NULL, 150, 8312, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL),
(2, 1, 'r', NULL, NULL, NULL, NULL, 'ppJ');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'then', 'NIL', 1, NULL, '', 3467, NULL, NULL, 'Zpw8pwRwqZ8', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, 1, '(voice)', 498, 1),
(2, 2, 3, 2, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, 2, 1, 1),
(3, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 3, 'hQQrIhh'),
(2, 3, 2, 3, NULL),
(3, 3, 1, 1, 'TMMT');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'Bulgaria', 't');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'Feee', 'MM'),
(2, 1, 1, '-Infinity', 'tmmHtdmqx-td'),
(3, 2, 1, 'ie''e''l7li87', '');
ROLLBACK;

-- ============================================================================
-- Generated database 055/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', NULL, NULL, NULL, NULL, 'none'),
(2, 'jy', '[de]', 277, NULL, NULL, 'if'),
(3, '00', NULL, 585, NULL, 'bDK', 'none');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '999'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'IKK--KKK-KK', NULL, NULL, 'W7TWWB', NULL, 'if', 'BM'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, '', NULL, NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 33, '44r4m6h', NULL, NULL),
(2, 'Voice Character', NULL, 3418, NULL, 'LPT1', '99');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'sp%psOsO%', NULL, 1, 2006, 6, '__dict__', 462, NULL, 11, NULL, NULL),
(2, '%cc%%', NULL, 1, 2005, NULL, NULL, 125, 63, NULL, NULL, NULL),
(3, 'if', NULL, 1, 2005, 287, NULL, NULL, 150, 8312, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL),
(2, 1, 'r', NULL, NULL, NULL, NULL, 'ppJ');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'then', 'NIL', 1, NULL, '', 3467, NULL, NULL, 'Zpw8pwRwqZ8', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, 1, '(voice)', 498, 1),
(2, 2, 3, 2, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, 2, 1, 1),
(3, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 3, 'hQQrIhh'),
(2, 3, 2, 3, NULL),
(3, 3, 1, 1, 'TMMT');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'Bulgaria', 't');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'Feee', 'MM'),
(2, 1, 1, '-Infinity', 'tmmHtdmqx-td'),
(3, 2, 1, 'ie''e''l7li87', '');
ROLLBACK;

-- ============================================================================
-- Generated database 056/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', NULL, NULL, NULL, NULL, 'none'),
(2, 'jy', '[de]', 277, NULL, NULL, 'if'),
(3, '00', NULL, 585, NULL, 'bDK', 'none');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '999'),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'IKK--KKK-KK', NULL, NULL, 'W7TWWB', NULL, 'if', 'BM'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, '', NULL, NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 33, '44r4m6h', NULL, NULL),
(2, 'Voice Character', NULL, 3418, NULL, 'LPT1', '99');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'sp%psOsO%', NULL, 1, 2006, 6, '__dict__', 462, NULL, 11, NULL, NULL),
(2, '%cc%%', NULL, 1, 2005, NULL, NULL, 125, 63, NULL, NULL, NULL),
(3, '1v28080Eu', NULL, 1, 2005, 287, NULL, 1, NULL, 8312, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL),
(2, 1, 'r', NULL, NULL, NULL, NULL, 'ppJ');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'then', 'NIL', 1, NULL, '', 3467, NULL, NULL, 'Zpw8pwRwqZ8', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, 1, '(voice)', 498, 1),
(2, 2, 3, 2, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1),
(2, 2, 1, 1),
(3, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 3, 'hQQrIhh'),
(2, 3, 2, 3, NULL),
(3, 3, 1, 1, 'TMMT');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'Bulgaria', 't');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'Feee', 'MM'),
(2, 1, 1, '-Infinity', 'tmmHtdmqx-td'),
(3, 2, 1, 'ie''e''l7li87', '');
ROLLBACK;

-- ============================================================================
-- Generated database 057/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'yffV', NULL, 6911, NULL, 'True', NULL),
(2, 'h', '[us]', NULL, NULL, 'if', 'QQ');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'true', NULL, NULL),
(2, 'Downey Jr., Robert', '-Infinity', NULL, '4_4cc11ckcckckkH', NULL, NULL, '', 'rFFFF1r'),
(3, 'Downey Jr., Robert', '-Infinity', NULL, 'x', NULL, NULL, NULL, 'h');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'None', NULL, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'uSiSQSiCCuSSuCQC', 'false', 2, NULL, 3, NULL, NULL, NULL, 340, NULL, ''),
(2, 'COM1', 'if', 2, 2009, 1104, 'T', 64, NULL, 395, NULL, '0');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '---', NULL, NULL, 'bGbRb0b  n0GGnRnbn 20002', '', ''),
(2, 1, '7Y77YYNY', 'C9J9exClzeC9exqzqC0JCeqeCC', 'k33', NULL, 'Inf', '6');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '%%%%%%%%%%%%%%%', '', 1, NULL, 'wwwwwwww', 494, NULL, NULL, NULL, 'GGGGGGG'),
(2, 1, '0', NULL, 1, NULL, NULL, 127, 129, 1807, 'Infinity', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 1, 1, NULL, 604, 1),
(2, 1, 1, 1, '(uncredited)', NULL, 2),
(3, 2, 1, NULL, NULL, 2287, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 3, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, 'NIL'),
(2, 2, 2, 2, ''),
(3, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', '11I0I0I');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '4.0', 'I9p9N');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 1),
(2, 1, 2, 1),
(3, 2, 2, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'True', 'wMMgMMgwM'),
(2, 1, 1, 'GmG', 'rddd');
ROLLBACK;

-- ============================================================================
-- Generated database 058/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'yffV', NULL, 6911, NULL, 'false', NULL),
(2, 'h', '[us]', NULL, NULL, 'if', 'QQ');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'true', NULL, NULL),
(2, 'Downey Jr., Robert', '-Infinity', NULL, '4_4cc11ckcckckkH', NULL, NULL, '', 'rFFFF1r'),
(3, 'Downey Jr., Robert', '-Infinity', NULL, 'x', NULL, NULL, NULL, 'h');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'None', NULL, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'uSiSQSiCCuSSuCQC', 'false', 2, NULL, 3, NULL, NULL, NULL, 340, NULL, ''),
(2, 'COM1', 'if', 2, 2009, 1104, 'T', 64, NULL, 395, NULL, '0');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '---', NULL, NULL, 'bGbRb0b  n0GGnRnbn 20002', '', ''),
(2, 1, '7Y77YYNY', 'C9J9exClzeC9exqzqC0JCeqeCC', 'k33', NULL, 'Inf', '6');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '%%%%%%%%%%%%%%%', '', 1, NULL, 'wwwwwwww', 494, NULL, NULL, NULL, 'GGGGGGG'),
(2, 1, '0', NULL, 1, NULL, NULL, 127, 129, 1807, 'Infinity', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 1, 1, NULL, 604, 1),
(2, 1, 1, 1, '(uncredited)', NULL, 2),
(3, 2, 1, NULL, NULL, 2287, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 3, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, 'NIL'),
(2, 2, 2, 2, ''),
(3, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', '11I0I0I');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '4.0', 'I9p9N');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 1),
(2, 1, 2, 1),
(3, 2, 2, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'True', 'wMMgMMgwM'),
(2, 1, 1, 'GmG', 'rddd');
ROLLBACK;

-- ============================================================================
-- Generated database 059/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'yffV', NULL, 6911, NULL, 'True', NULL),
(2, 'h', '[us]', NULL, NULL, 'if', 'QQ');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'true', NULL, NULL),
(2, 'Downey Jr., Robert', '-Infinity', NULL, NULL, NULL, NULL, NULL, NULL),
(3, 'Other Person', '', NULL, NULL, NULL, '-Infinity', NULL, 'x');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'h', NULL, NULL, NULL, 'None');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', NULL, 2, NULL, NULL, 'false', NULL, NULL, 3, NULL, NULL),
(2, '1', NULL, 2, NULL, 1, NULL, 1, NULL, NULL, '1', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'T', '1', '1', '0', NULL, NULL),
(2, 2, '1', 'bGbRb0b  n0GGnRnbn 20002', '', '', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 2, 1, NULL, NULL, 1, NULL, '6', NULL),
(2, 2, '1', '1', 2, NULL, NULL, 494, NULL, NULL, NULL, 'GGGGGGG');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, NULL, NULL, NULL, 2),
(2, 2, 2, NULL, '(voice)', 1807, 2),
(3, 2, 2, NULL, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 2, 1, 2, NULL),
(3, 2, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '4.0', NULL),
(2, 2, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 1),
(2, 2, 1),
(3, 2, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1),
(2, 1, 2, 2),
(3, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '1', NULL),
(2, 2, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 060/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'yffV', NULL, 6911, NULL, 'True', NULL),
(2, 'h', '[us]', NULL, NULL, 'if', 'QQ');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'true', NULL, NULL),
(2, 'Downey Jr., Robert', '-Infinity', NULL, '4_4cc11ckcckckkH', NULL, NULL, '', 'rFFFF1r'),
(3, 'Downey Jr., Robert', '-Infinity', NULL, 'x', NULL, NULL, NULL, 'h');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'None', NULL, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'uSiSQSiCCuSSuCQC', 'false', 2, NULL, 3, NULL, NULL, NULL, 340, NULL, '');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, NULL, NULL, '1', NULL),
(2, 2, 'T', '1', '1', '0', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '1', 'bGbRb0b  n0GGnRnbn 20002', 2, 1, '1', NULL, 1, NULL, 'k33', NULL),
(2, 1, 'Inf', '6', 1, NULL, NULL, 1, NULL, NULL, 'wwwwwwww', '1');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, 1, NULL, NULL, 2),
(2, 2, 1, NULL, NULL, NULL, 2),
(3, 2, 1, NULL, '(voice)', 1, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, '1'),
(2, 1, 1, 2, NULL),
(3, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', '1'),
(2, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 2),
(3, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '1', NULL),
(2, 2, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 061/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ttSCtAC--t', '[us]', NULL, NULL, NULL, NULL),
(2, 'jDW66Xv6', NULL, 128, '444QQQQQ', NULL, NULL),
(3, 'zk77K7Kq7q7K', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '__dict__', NULL, NULL, 'a', 'iiQQOQu', 'gmy8A88og8', '22');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'MR4', 'ddd', NULL),
(2, 'Voice Character', NULL, 370, '-Sf-%', 'o-o-o-ooo', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'hF   hhF', NULL, 2, 2006, 1, 'LDNuLu_L--__DEfEDuLELuD_u', 512, 333, NULL, NULL, 'ttt'),
(2, 'qCqqkA_KZAC_qAq__kKQ', NULL, 2, NULL, NULL, NULL, 1688, 501, 286, NULL, NULL),
(3, 'n9y', NULL, 1, NULL, NULL, '', 250, 285, 329, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'CKbCK', 'bOCb', 'Hs2ie', 'mmmmm', NULL, 'THJTJITJJHHJ_NNJT'),
(2, 1, 'LLLLLLLLLLLLLLLLLL', 'F', '1e100', '', 'mWlmlllllmlW', 'if'),
(3, 1, 'p', 'none', NULL, NULL, NULL, 'yWyWWyWWy');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', 'vDNP2z', 1, 239, NULL, NULL, 1038, 1465, NULL, NULL),
(2, 3, '-Infinity', NULL, 1, NULL, 'L', 892, NULL, NULL, NULL, 'z X ');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 2, '(uncredited)', NULL, 1),
(2, 1, 1, NULL, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, 2, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 2, 1, 1, NULL),
(3, 2, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL),
(2, 3, 3, 'USA', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 3, 3, '8.0', NULL),
(2, 3, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'RIRR', 'y6ym9syRRm');
ROLLBACK;

-- ============================================================================
-- Generated database 062/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ttSCtAC--t', '[us]', NULL, NULL, NULL, NULL),
(2, 'jDW66Xv6', NULL, 128, '444QQQQQ', NULL, NULL),
(3, 'zk77K7Kq7q7K', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '__dict__', NULL, NULL, 'a', 'iiQQOQu', 'gmy8A88og8', '22');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'MR4', 'ddd', NULL),
(2, 'Voice Character', NULL, 370, '-Sf-%', 'o-o-o-ooo', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'hF   hhF', NULL, 2, 2006, 1, 'LDNuLu_L--__DEfEDuLELuD_u', 512, 333, NULL, NULL, 'ttt'),
(2, 'qCqqkA_KZAC_qAq__kKQ', NULL, 2, NULL, NULL, NULL, 1688, 501, 286, NULL, NULL),
(3, 'n9y', NULL, 1, NULL, NULL, '', 250, 285, 329, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'CKbCK', 'bOCb', 'Hs2ie', 'mmmmm', NULL, 'THJTJITJJHHJ_NNJT'),
(2, 1, 'LLLLLLLLLLLLLLLLLL', 'F', '1e100', '', 'mWlmlllllmlW', 'if'),
(3, 1, 'p', 'none', NULL, NULL, NULL, 'yWyWWyWWy');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', 'vDNP2z', 1, 239, NULL, NULL, 1038, 1465, NULL, NULL),
(2, 3, 'if', NULL, 1, NULL, 'L', 892, NULL, NULL, NULL, 'z X ');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 2, '(uncredited)', NULL, 1),
(2, 1, 1, NULL, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, 2, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 2, 1, 1, NULL),
(3, 2, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL),
(2, 3, 3, 'USA', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 3, 3, '8.0', NULL),
(2, 3, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'RIRR', 'y6ym9syRRm');
ROLLBACK;

-- ============================================================================
-- Generated database 063/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ttSCtAC--t', '[us]', NULL, NULL, NULL, NULL),
(2, 'jDW66Xv6', NULL, 128, '444QQQQQ', NULL, NULL),
(3, 'zk77K7Kq7q7K', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', '__dict__', NULL, NULL, 'a', 'iiQQOQu', 'gmy8A88og8', '22');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'MR4', 'ddd', NULL),
(2, 'Voice Character', NULL, 370, '-Sf-%', 'o-o-o-ooo', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'hF   hhF', NULL, 2, 2006, 1, 'LDNuLu_L--__DEfEDuLELuD_u', 512, 333, NULL, NULL, 'ttt'),
(2, 'qCqqkA_KZAC_qAq__kKQ', NULL, 2, NULL, NULL, NULL, 1688, 501, 286, NULL, NULL),
(3, 'n9y', NULL, 1, NULL, NULL, '', 250, 285, 329, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'CKbCK', 'bOCb', 'Hs2ie', 'mmmmm', NULL, 'THJTJITJJHHJ_NNJT'),
(2, 1, 'LLLLLLLLLLLLLLLLLL', 'F', '1e100', '', 'mWlmlllllmlW', 'if'),
(3, 1, 'p', 'none', NULL, NULL, NULL, 'yWyWWyWWy');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', 'vDNP2z', 1, 239, NULL, NULL, 1038, 1465, NULL, NULL),
(2, 3, '-Infinity', NULL, 1, NULL, 'L', 892, NULL, NULL, NULL, 'z X ');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 2, '(uncredited)', NULL, 1),
(2, 1, 1, NULL, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 3, 2),
(2, 2, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 2, 1, 1, NULL),
(3, 2, 1, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL),
(2, 3, 3, 'USA', '');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 3, 3, '8.0', NULL),
(2, 3, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'RIRR', 'y6ym9syRRm');
ROLLBACK;

-- ============================================================================
-- Generated database 064/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[ru]', 4565, NULL, '  ', '-------------------'),
(2, '4', '[us]', 10000, NULL, '__proto__', '2-O2O'),
(3, '0', '[us]', 9999, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''),
(2, 'hero-sequel', NULL),
(3, 'hero-sequel', '');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 2336, NULL, '_mII''''', 'True', NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'LPT1', NULL, NULL, NULL, 'INF'),
(2, 'Voice Character', 'LPT1', 641, 'null', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'l', '', 1, 2005, 1674, NULL, NULL, NULL, 9407, 'then', NULL),
(2, 'TTTTTTTTT', 'ff', 1, NULL, 6655, 'FqWooWqolWWRW', NULL, 665, 702, NULL, 'ezz'),
(3, '00gag', NULL, 2, 2007, 461, NULL, 10000, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'NULL', NULL, NULL, 'NaN', 'Khhh', NULL),
(2, 1, '5p0v55vv0p', 'qSSSS', '9xsu', '333', '', '--NNN');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'wK-', '_z3', 2, 3130, NULL, NULL, 4736, NULL, '7s', NULL),
(2, 2, 'kKmkkEmBkmh', 'ulLglLuuyyullYYYYYgYuuYYyuYYYllg', 1, NULL, NULL, NULL, 6472, NULL, NULL, 'Anrh'),
(3, 2, 'P', NULL, 2, 323, NULL, 590, 144, 124, '', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2),
(2, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, '0'),
(2, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'USA', NULL),
(2, 3, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 3),
(2, 1, 2),
(3, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 2, 2),
(3, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'NaN', '78IhB-');
ROLLBACK;

-- ============================================================================
-- Generated database 065/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[ru]', 4565, NULL, '  ', '-------------------'),
(2, '4', '[us]', 10000, NULL, '__proto__', '2-O2O'),
(3, '0', '[us]', 9999, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''),
(2, 'hero-sequel', NULL),
(3, 'hero-sequel', '');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 2336, NULL, '_mII''''', 'True', NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'LPT1', NULL, NULL, NULL, 'INF'),
(2, 'Voice Character', 'LPT1', 641, 'null', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'l', '', 1, 2005, 1674, NULL, NULL, NULL, 9407, 'then', NULL),
(2, 'TTTTTTTTT', 'ff', 1, NULL, 6655, 'FqWooWqolWWRW', NULL, 665, 702, NULL, 'ezz'),
(3, '00gag', NULL, 2, 2007, 461, NULL, 2, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'NULL', NULL, NULL, 'NaN', 'Khhh', NULL),
(2, 1, '5p0v55vv0p', 'qSSSS', '9xsu', '333', '', '--NNN');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'wK-', '_z3', 2, 3130, NULL, NULL, 4736, NULL, '7s', NULL),
(2, 2, 'kKmkkEmBkmh', 'ulLglLuuyyullYYYYYgYuuYYyuYYYllg', 1, NULL, NULL, NULL, 6472, NULL, NULL, 'Anrh'),
(3, 2, 'P', NULL, 2, 323, NULL, 590, 144, 124, '', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2),
(2, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, '0'),
(2, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'USA', NULL),
(2, 3, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 3),
(2, 1, 2),
(3, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 2, 2),
(3, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'NaN', '78IhB-');
ROLLBACK;

-- ============================================================================
-- Generated database 066/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[ru]', 4565, NULL, '  ', '-------------------'),
(2, '4', '[us]', 10000, NULL, '__proto__', '2-O2O'),
(3, '0', '[us]', 9999, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''),
(2, 'hero-sequel', NULL),
(3, 'hero-sequel', '');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 2336, NULL, '_mII''''', 'True', NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'LPT1', NULL, NULL, NULL, 'INF'),
(2, 'Voice Character', 'LPT1', 641, 'null', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'l', '', 1, 2005, 1674, NULL, NULL, NULL, 9407, 'then', NULL),
(2, 'TTTTTTTTT', 'ff', 1, NULL, 6655, 'FqWooWqolWWRW', NULL, 665, 702, NULL, 'ezz'),
(3, 'INF', NULL, 2, 2007, 461, NULL, 10000, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'NULL', NULL, NULL, 'NaN', 'Khhh', NULL),
(2, 1, '5p0v55vv0p', 'qSSSS', '9xsu', '333', '', '--NNN');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'wK-', '_z3', 2, 3130, NULL, NULL, 4736, NULL, '7s', NULL),
(2, 2, 'kKmkkEmBkmh', 'ulLglLuuyyullYYYYYgYuuYYyuYYYllg', 1, NULL, NULL, NULL, 6472, NULL, NULL, 'Anrh'),
(3, 2, 'P', NULL, 2, 323, NULL, 590, 144, 124, '', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2),
(2, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, '0'),
(2, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'USA', NULL),
(2, 3, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 3),
(2, 1, 2),
(3, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 2, 2),
(3, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'NaN', '78IhB-');
ROLLBACK;

-- ============================================================================
-- Generated database 067/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[ru]', 4565, NULL, '  ', '-------------------'),
(2, '4', '[us]', 10000, NULL, '__proto__', '2-O2O'),
(3, '0', '[us]', 9999, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''),
(2, 'hero-sequel', NULL),
(3, 'hero-sequel', '');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 2336, NULL, '_mII''''', 'True', NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'LPT1', NULL, NULL, NULL, 'INF'),
(2, 'Voice Character', 'LPT1', 641, 'null', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'l', '', 1, 2005, 1674, NULL, NULL, NULL, 9407, 'then', NULL),
(2, 'TTTTTTTTT', 'ff', 1, NULL, 6655, 'FqWooWqolWWRW', NULL, 665, 702, NULL, 'ezz'),
(3, '00gag', NULL, 2, 2007, 461, NULL, 10000, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'NULL', NULL, NULL, 'NaN', 'Khhh', NULL),
(2, 1, '5p0v55vv0p', 'qSSSS', '9xsu', '333', '', '--NNN');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'wK-', '_z3', 2, 3130, NULL, NULL, 4736, NULL, '7s', NULL),
(2, 2, 'kKmkkEmBkmh', '2-O2O', 1, NULL, NULL, NULL, 6472, NULL, NULL, 'Anrh'),
(3, 2, 'P', NULL, 2, 323, NULL, 590, 144, 124, '', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2),
(2, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, '0'),
(2, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'USA', NULL),
(2, 3, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 3),
(2, 1, 2),
(3, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 2, 2),
(3, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'NaN', '78IhB-');
ROLLBACK;

-- ============================================================================
-- Generated database 068/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[ru]', 4565, NULL, '  ', '-------------------'),
(2, '4', '[us]', 10000, NULL, '__proto__', '2-O2O'),
(3, '0', '[us]', 9999, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', ''),
(2, 'hero-sequel', NULL),
(3, 'hero-sequel', '');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, 2336, NULL, '_mII''''', 'True', NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'LPT1', NULL, NULL, NULL, 'INF'),
(2, 'Voice Character', 'LPT1', 641, 'null', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'l', '', 1, 2005, 1674, NULL, NULL, NULL, 9407, 'then', NULL),
(2, 'TTTTTTTTT', 'ff', 1, NULL, 6655, 'FqWooWqolWWRW', NULL, 665, 702, NULL, 'ezz'),
(3, '00gag', NULL, 2, 2007, 461, NULL, 10000, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'NULL', NULL, NULL, 'NaN', 'Khhh', NULL),
(2, 1, '5p0v55vv0p', 'qSSSS', '9xsu', '333', '', 'INF');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'wK-', '_z3', 2, 3130, NULL, NULL, 4736, NULL, '7s', NULL),
(2, 2, 'kKmkkEmBkmh', 'ulLglLuuyyullYYYYYgYuuYYyuYYYllg', 1, NULL, NULL, NULL, 6472, NULL, NULL, 'Anrh'),
(3, 2, 'P', NULL, 2, 323, NULL, 590, 144, 124, '', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 2),
(2, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, '0'),
(2, 1, 2, 1, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'USA', NULL),
(2, 3, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 3),
(2, 1, 2),
(3, 3, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 2, 1),
(2, 2, 2, 2),
(3, 2, 3, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'NaN', '78IhB-');
ROLLBACK;

-- ============================================================================
-- Generated database 069/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[de]', 69, 'iimmi', 'fqEM', 'Jbaf9aRnJRfRaRfH''R''fabnJRbnfHH9'),
(2, '999999999999999999999999999999', '[ru]', NULL, 'roorr8rooo8oor88r8oooo8o888o88ro', 'E', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '%hm'),
(2, 'marvel-cinematic-universe', 'p3pLangp'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '-', 2705, '8888___8', '_e__eeee_____', NULL, '', NULL),
(2, 'Other Person', '', NULL, 'tW', 'None', NULL, 'ey55', NULL),
(3, 'Downey Jr., Robert', NULL, 1014, 'bY-Y-bbb-Y-', NULL, 'INF', '', 'D');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, NULL, NULL, '-'),
(2, 'Voice Character', 'AwAwAj22Ajww', 635, '', NULL, 'uu');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'twtt', NULL, 1, NULL, NULL, NULL, NULL, NULL, 2179, NULL, 'NIL'),
(2, 'heh', NULL, 1, 2007, 9900, 'Y', 1249, 193, NULL, NULL, NULL),
(3, 'k', 'YU3%JU', 1, 2012, NULL, NULL, NULL, 4039, 849, NULL, 'eeeeeeeeeeeeeeeeee');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, '', 'DGDjGK', NULL, '2u2XuIJuJJuIAJXV2AI', 'None', 'm4'),
(2, 1, 'jjjjjjj', 'null', '7susu', '', 'PgPZPPKPPK', NULL),
(3, 1, 'qqqqqqqqqqqqqqqqqqq', 'FALSE', 'cX', 'ppBp_bpBBB', NULL, '0');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 3, 'uuuFtuFtuFutF', 'Pamm9a9E9', 1, 4186, NULL, NULL, 1571, 223, NULL, 'TTU'),
(2, 3, 'R', NULL, 1, 3568, NULL, NULL, NULL, NULL, NULL, NULL),
(3, 2, '1e100', NULL, 1, NULL, 'JF212F', NULL, 999, 2474, 'False', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, 2, '(voice)', NULL, 2),
(2, 2, 1, NULL, '(uncredited)', 1546, 3),
(3, 2, 3, 2, '(voice)', 1580, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1),
(2, NULL, 2, 2),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, NULL),
(2, 3, 2, 1, '__proto__'),
(3, 1, 2, 1, 'xI');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', ''),
(2, 1, 1, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '8.0', NULL),
(2, 2, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 3, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '''''''''''''''', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 070/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[de]', 69, 'iimmi', 'fqEM', 'Jbaf9aRnJRfRaRfH''R''fabnJRbnfHH9'),
(2, '999999999999999999999999999999', '[ru]', NULL, 'roorr8rooo8oor88r8oooo8o888o88ro', 'E', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '%hm'),
(2, 'marvel-cinematic-universe', 'p3pLangp'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '-', 2705, '8888___8', '_e__eeee_____', NULL, '', NULL),
(2, 'Other Person', '', NULL, 'tW', 'None', NULL, 'ey55', NULL),
(3, 'Downey Jr., Robert', NULL, 1014, 'bY-Y-bbb-Y-', NULL, 'INF', '', 'D');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, NULL, NULL, '-'),
(2, 'Voice Character', 'AwAwAj22Ajww', 635, '', NULL, 'uu');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'twtt', NULL, 1, NULL, NULL, NULL, NULL, NULL, 2179, NULL, 'NIL'),
(2, 'heh', NULL, 1, 2007, 9900, 'Y', 1249, 193, NULL, NULL, NULL),
(3, 'k', 'YU3%JU', 1, 2012, NULL, NULL, NULL, 4039, 849, NULL, 'eeeeeeeeeeeeeeeeee');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, '', 'DGDjGK', NULL, '2u2XuIJuJJuIAJXV2AI', 'None', 'm4'),
(2, 1, 'jjjjjjj', 'null', '7susu', '', 'PgPZPPKPPK', NULL),
(3, 1, 'qqqqqqqqqqqqqqqqqqq', 'FALSE', 'cX', 'ppBp_bpBBB', NULL, '0');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 3, 'uuuFtuFtuFutF', 'Pamm9a9E9', 1, 4186, NULL, NULL, 1571, 223, NULL, 'TTU'),
(2, 3, 'R', NULL, 1, 3568, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, 999, 2),
(3, 2, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, 1, 2, 2),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 2, NULL),
(2, 3, 2, 2, '1'),
(3, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL),
(2, 2, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', NULL),
(2, 2, 2, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 071/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[de]', 69, 'iimmi', 'fqEM', 'Jbaf9aRnJRfRaRfH''R''fabnJRbnfHH9'),
(2, '999999999999999999999999999999', '[ru]', NULL, 'roorr8rooo8oor88r8oooo8o888o88ro', 'E', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '%hm'),
(2, 'marvel-cinematic-universe', 'p3pLangp'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '-', 2705, '8888___8', '_e__eeee_____', NULL, '', NULL),
(2, 'Other Person', '', NULL, 'tW', 'None', NULL, 'ey55', NULL),
(3, 'Downey Jr., Robert', NULL, 1014, 'bY-Y-bbb-Y-', NULL, 'INF', '', 'D');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, NULL, NULL, '-'),
(2, 'Voice Character', 'AwAwAj22Ajww', 635, '', NULL, 'uu');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'twtt', NULL, 1, NULL, NULL, NULL, NULL, NULL, 2179, NULL, 'NIL'),
(2, 'heh', NULL, 1, 2007, 9900, 'Y', 1249, 193, NULL, NULL, NULL),
(3, 'k', 'YU3%JU', 1, 2012, NULL, NULL, NULL, 4039, 849, NULL, 'eeeeeeeeeeeeeeeeee');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, '', 'DGDjGK', NULL, '2u2XuIJuJJuIAJXV2AI', NULL, NULL),
(2, 2, '', NULL, NULL, NULL, NULL, NULL),
(3, 2, 'null', '7susu', '', 'PgPZPPKPPK', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '1', NULL, 2, 1, NULL, 1, NULL, NULL, '0', NULL),
(2, 2, '1', NULL, 2, NULL, '1', NULL, 1571, 223, NULL, 'TTU'),
(3, 3, 'R', NULL, 1, 3568, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 2, NULL, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, 999, 2),
(3, 2, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, 1, 2, 2),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 2, NULL),
(2, 3, 2, 2, '1'),
(3, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', NULL),
(2, 2, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', NULL),
(2, 2, 2, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 072/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[de]', 69, 'iimmi', 'fqEM', 'Jbaf9aRnJRfRaRfH''R''fabnJRbnfHH9'),
(2, '999999999999999999999999999999', '[ru]', NULL, 'roorr8rooo8oor88r8oooo8o888o88ro', 'E', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '%hm'),
(2, 'marvel-cinematic-universe', 'p3pLangp'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '-', 2705, '8888___8', '_e__eeee_____', NULL, '', NULL),
(2, 'Other Person', '', NULL, 'tW', 'None', NULL, 'ey55', NULL),
(3, 'Downey Jr., Robert', NULL, 1014, 'bY-Y-bbb-Y-', NULL, 'INF', '', 'D');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, NULL, NULL, '-'),
(2, 'Voice Character', 'AwAwAj22Ajww', 635, '', NULL, 'uu');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'twtt', NULL, 1, NULL, NULL, NULL, NULL, NULL, 2179, NULL, 'NIL'),
(2, 'heh', NULL, 1, 2007, 9900, 'Y', 1249, 193, NULL, NULL, NULL),
(3, 'k', 'YU3%JU', 1, 2012, NULL, NULL, NULL, 4039, 849, NULL, 'eeeeeeeeeeeeeeeeee');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, '', 'DGDjGK', NULL, '2u2XuIJuJJuIAJXV2AI', 'None', 'm4'),
(2, 1, 'jjjjjjj', 'null', '7susu', '', 'PgPZPPKPPK', NULL),
(3, 1, 'qqqqqqqqqqqqqqqqqqq', 'FALSE', 'cX', 'Y', NULL, '0');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 3, 'uuuFtuFtuFutF', 'Pamm9a9E9', 1, 4186, NULL, NULL, 1571, 223, NULL, 'TTU'),
(2, 3, 'R', NULL, 1, 3568, NULL, NULL, NULL, NULL, NULL, NULL),
(3, 2, '1e100', NULL, 1, NULL, 'JF212F', NULL, 999, 2474, 'False', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 3, 2, '(voice)', NULL, 2),
(2, 2, 1, NULL, '(uncredited)', 1546, 3),
(3, 2, 3, 2, '(voice)', 1580, 3);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1),
(2, NULL, 2, 2),
(3, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 3, 2, 2, NULL),
(2, 3, 2, 1, '__proto__'),
(3, 1, 2, 1, 'xI');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'Bulgaria', ''),
(2, 1, 1, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '8.0', NULL),
(2, 2, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 3, 3, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 2, '''''''''''''''', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 073/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified'),
(3, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[de]', 69, 'iimmi', 'fqEM', 'Jbaf9aRnJRfRaRfH''R''fabnJRbnfHH9'),
(2, '999999999999999999999999999999', '[ru]', 1, NULL, NULL, 'E');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, NULL, NULL, NULL, NULL, '-', '1'),
(2, 'Other Person', NULL, 1, NULL, NULL, '', NULL, NULL),
(3, 'Other Person', '1', NULL, NULL, 'None', NULL, 'ey55', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', 'bY-Y-bbb-Y-', NULL, 'INF', '', 'D'),
(2, 'Voice Character', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', NULL, 2, NULL, NULL, NULL, NULL, 1, NULL, '1', '1'),
(2, '1', NULL, 2, NULL, NULL, NULL, NULL, NULL, NULL, '1', 'NIL'),
(3, 'heh', NULL, 1, 2007, 9900, 'Y', 1249, 193, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '', NULL, NULL, NULL, '1', NULL),
(2, 2, '', NULL, '1', 'eeeeeeeeeeeeeeeeee', NULL, ''),
(3, 2, '1', '2u2XuIJuJJuIAJXV2AI', 'None', 'm4', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '', NULL, 2, 1, NULL, 1, 1, NULL, NULL, NULL),
(2, 2, '1', NULL, 2, 1, NULL, 1, NULL, NULL, '0', NULL),
(3, 2, '1', NULL, 2, NULL, '1', NULL, 1571, 223, NULL, 'TTU');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 2, NULL, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, NULL, 2),
(3, 2, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2),
(2, NULL, 2, 2),
(3, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 2, 2, NULL),
(2, 1, 2, 2, NULL),
(3, 2, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'USA', '1'),
(2, 2, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', NULL),
(2, 2, 2, '4.0', '1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 3, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 074/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ch X6Xh h6Q X 6', NULL, 3395, 'yT699y', 'qqqqqq', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'bbv'),
(3, 'hero-sequel', 'null');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 290, NULL, 'bbbbbbbbbbbbbb', 'mt', 'h', 'undefined'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'u', '6ssml', 'COM1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 3926, NULL, 'J22Jbb', 'COM1'),
(2, 'Other Character', NULL, NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__z_11z', NULL, 2, 2008, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Infinity', NULL, 2, 2006, NULL, 'Ezz5MMzz', 2048, 2360, NULL, NULL, 'if'),
(3, 'gy-ggmhgm--VVm-ggh-gy', NULL, 1, NULL, NULL, 'TRUE', NULL, 9999, 624, '8r', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '__proto__', 'FALSE', 'Infinity', NULL, '1e100', 'NUL'),
(2, 2, '', 'Infinity', NULL, NULL, NULL, NULL),
(3, 2, '3w85w8', NULL, 'UUUuU', 'ZOXA-AOFXY-OHXA-AFYFOFHOO', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '9', NULL, 1, 1045, 'Scunthorpe', NULL, 886, NULL, 'KKOAOF', NULL),
(2, 3, '_kk', NULL, 1, NULL, 'rrmrYmrrrYN', NULL, 1892, 304, 'false', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 2, NULL, 53, 1),
(2, 2, 3, 1, NULL, NULL, 1),
(3, 1, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 3, 1, 1, 'Rhddh');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', ''),
(2, 2, 2, 'USA', NULL),
(3, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 3, 2, '4.0', 'JJJ'''''),
(3, 1, 3, '8.0', 'HgnW');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'jGZAZvrZYbYprArYr', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 075/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ch X6Xh h6Q X 6', NULL, 3395, 'yT699y', 'qqqqqq', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'bbv'),
(3, 'hero-sequel', 'null');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 290, NULL, 'bbbbbbbbbbbbbb', 'mt', 'h', 'undefined'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'u', '6ssml', 'COM1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 3926, NULL, 'J22Jbb', 'COM1'),
(2, 'Other Character', NULL, NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__z_11z', NULL, 2, 2008, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Infinity', NULL, 2, 2006, NULL, 'Ezz5MMzz', 2048, 2360, NULL, NULL, 'if'),
(3, 'gy-ggmhgm--VVm-ggh-gy', NULL, 1, NULL, NULL, 'TRUE', NULL, 9999, 624, '8r', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '__proto__', 'FALSE', 'Infinity', NULL, '1e100', 'NUL'),
(2, 2, '', 'Infinity', NULL, NULL, NULL, NULL),
(3, 2, '3w85w8', NULL, 'UUUuU', 'ZOXA-AOFXY-OHXA-AFYFOFHOO', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '9', NULL, 1, 1045, 'Scunthorpe', NULL, 886, NULL, 'KKOAOF', NULL),
(2, 3, '_kk', NULL, 1, NULL, 'rrmrYmrrrYN', NULL, 1892, 304, 'false', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 2, NULL, 53, 1),
(2, 2, 3, 1, NULL, NULL, 1),
(3, 1, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 3, 1, 1, 'Rhddh');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', ''),
(2, 2, 2, 'USA', NULL),
(3, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 3, 2, '4.0', 'JJJ'''''),
(3, 1, 3, '8.0', 'HgnW');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1),
(2, 2, 1, 2),
(3, 2, 2, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'yuMiu881TVMyMy8uiGyyu', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 076/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ch X6Xh h6Q X 6', NULL, 3395, 'yT699y', 'qqqqqq', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'bbv'),
(3, 'hero-sequel', 'null');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 290, NULL, 'bbbbbbbbbbbbbb', 'gy-ggmhgm--VVm-ggh-gy', 'h', 'undefined'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'u', '6ssml', 'COM1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 3926, NULL, 'J22Jbb', 'COM1'),
(2, 'Other Character', NULL, NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__z_11z', NULL, 2, 2008, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Infinity', NULL, 2, 2006, NULL, 'Ezz5MMzz', 2048, 2360, NULL, NULL, 'if'),
(3, 'gy-ggmhgm--VVm-ggh-gy', NULL, 1, NULL, NULL, 'TRUE', NULL, 9999, 624, '8r', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '__proto__', 'FALSE', 'Infinity', NULL, '1e100', 'NUL'),
(2, 2, '', 'Infinity', NULL, NULL, NULL, NULL),
(3, 2, '3w85w8', NULL, 'UUUuU', 'ZOXA-AOFXY-OHXA-AFYFOFHOO', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '9', NULL, 1, 1045, 'Scunthorpe', NULL, 886, NULL, 'KKOAOF', NULL),
(2, 3, '_kk', NULL, 1, NULL, 'rrmrYmrrrYN', NULL, 1892, 304, 'false', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 2, NULL, 53, 1),
(2, 2, 3, 1, NULL, NULL, 1),
(3, 1, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 3, 1, 1, 'Rhddh');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', ''),
(2, 2, 2, 'USA', NULL),
(3, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 3, 2, '4.0', 'JJJ'''''),
(3, 1, 3, '8.0', 'HgnW');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'jGZAZvrZYbYprArYr', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 077/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ch X6Xh h6Q X 6', NULL, 3395, 'yT699y', 'qqqqqq', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'bbv'),
(3, 'hero-sequel', 'null');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 290, NULL, 'bbbbbbbbbbbbbb', 'mt', 'h', 'undefined'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'u', '6ssml', 'COM1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 3926, NULL, 'J22Jbb', 'COM1'),
(2, 'Other Character', NULL, NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__z_11z', NULL, 2, 2008, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Infinity', NULL, 2, 2006, NULL, 'Ezz5MMzz', 2048, 2360, NULL, NULL, 'if'),
(3, 'gy-ggmhgm--VVm-ggh-gy', NULL, 1, NULL, NULL, 'TRUE', NULL, 9999, 624, '8r', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '__proto__', 'FALSE', 'Infinity', NULL, '', NULL),
(2, 2, 'NUL', NULL, '', NULL, NULL, NULL),
(3, 2, '1', NULL, NULL, NULL, NULL, 'UUUuU');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'ZOXA-AOFXY-OHXA-AFYFOFHOO', NULL, 2, NULL, NULL, NULL, NULL, NULL, '1', NULL),
(2, 2, '1', NULL, 2, 1, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 2, NULL, NULL, 2),
(2, 2, 2, NULL, '(voice)', NULL, 2),
(3, 1, 2, 2, NULL, 53, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 1, 'USA', NULL),
(2, 1, 2, 'USA', NULL),
(3, 2, 3, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 2, '4.0', NULL),
(2, 2, 1, '4.0', '1'),
(3, 1, 2, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 078/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ch X6Xh h6Q X 6', NULL, 3395, 'yT699y', 'qqqqqq', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'bbv'),
(3, 'hero-sequel', 'null');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 290, NULL, 'bbbbbbbbbbbbbb', 'mt', 'h', 'undefined'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'u', '6ssml', 'COM1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 3926, NULL, 'J22Jbb', 'COM1'),
(2, 'Other Character', NULL, NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__z_11z', NULL, 2, 2008, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Infinity', NULL, 2, 2006, NULL, 'Ezz5MMzz', 2048, 2360, NULL, NULL, 'if'),
(3, 'gy-ggmhgm--VVm-ggh-gy', NULL, 1, NULL, NULL, 'TRUE', NULL, 9999, 624, '8r', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '__proto__', 'FALSE', 'Infinity', NULL, '1e100', 'NUL'),
(2, 2, '', 'Infinity', NULL, NULL, NULL, NULL),
(3, 2, '3w85w8', NULL, 'UUUuU', 'ZOXA-AOFXY-OHXA-AFYFOFHOO', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '9', NULL, 1, 1045, 'Scunthorpe', NULL, 886, NULL, 'KKOAOF', NULL),
(2, 3, '_kk', NULL, 1, NULL, 'rrmrYmrrrYN', NULL, 1892, 304, 'false', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 2, NULL, 53, 1),
(2, 2, 3, 1, NULL, NULL, 1),
(3, 1, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 3, 1, 1, 'Ezz5MMzz');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', ''),
(2, 2, 2, 'USA', NULL),
(3, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 3, 2, '4.0', 'JJJ'''''),
(3, 1, 3, '8.0', 'HgnW');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'jGZAZvrZYbYprArYr', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 079/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'ch X6Xh h6Q X 6', NULL, 3395, 'yT699y', 'qqqqqq', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'distributors'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'bbv'),
(3, 'hero-sequel', 'null');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 290, NULL, 'bbbbbbbbbbbbbb', 'mt', 'h', 'undefined'),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'u', '6ssml', 'COM1');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actress'),
(3, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 3926, NULL, 'J22Jbb', 'COM1'),
(2, 'Other Character', NULL, NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '__z_11z', NULL, 2, 2008, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Infinity', NULL, 2, 2006, NULL, 'Ezz5MMzz', 2048, 2360, NULL, NULL, 'if'),
(3, 'gy-ggmhgm--VVm-ggh-gy', NULL, 1, NULL, NULL, 'TRUE', NULL, 9999, 624, '8r', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '__proto__', 'FALSE', 'Infinity', NULL, '1e100', 'NUL'),
(2, 2, '', 'Infinity', NULL, NULL, NULL, NULL),
(3, 2, '3w85w8', NULL, 'UUUuU', 'ZOXA-AOFXY-OHXA-AFYFOFHOO', NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '9', NULL, 1, 1045, 'UUUuU', NULL, 886, NULL, 'KKOAOF', NULL),
(2, 3, '_kk', NULL, 1, NULL, 'rrmrYmrrrYN', NULL, 1892, 304, 'false', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 2, 2, NULL, 53, 1),
(2, 2, 3, 1, NULL, NULL, 1),
(3, 1, 2, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 2, 1, 1, NULL),
(2, 3, 1, 1, 'Rhddh');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 2, 2, 'USA', ''),
(2, 2, 2, 'USA', NULL),
(3, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 3, 2, '4.0', 'JJJ'''''),
(3, 1, 3, '8.0', 'HgnW');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 2, 2, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'jGZAZvrZYbYprArYr', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 080/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'Inf', NULL, 3167, NULL, 'vX%XXGX', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '777TFFT', 8, NULL, NULL, NULL, 'GIg', 'qq'),
(2, 'Other Person', 'T', NULL, NULL, 'BQgJgJJQ', 'oxa', NULL, ''),
(3, 'Downey Jr., Robert', 'BBBF', NULL, 'OE', 'IK', 'ZZTTZTTZZZtZt', 'SSCY', 'gqgn');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'k', '', 'else'),
(2, 'Voice Character', 'nil', NULL, 'ZZZ', '000XX00', 'eey'),
(3, 'Other Character', NULL, NULL, '0', 'V4VV', 'undefined');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'bss', NULL, 2, 2005, NULL, NULL, 63, NULL, 122, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'then', '', NULL, NULL, NULL, 'ZZZQQtYZQZZZYZQ'),
(2, 2, ' 2', 'QHfQQvQHv', 'vKVVVKK', NULL, NULL, NULL),
(3, 2, '236', NULL, NULL, NULL, 'uM3', 'IbubuuqIbbbIbIIqqIbqu');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Q    _QS  _Q _ ', NULL, 2, 4758, 'GH', NULL, NULL, NULL, '-Infinity', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 1, 2, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'c'),
(2, 1, 2, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'Infinity', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 081/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'Inf', NULL, 3167, NULL, 'vX%XXGX', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '777TFFT', 8, NULL, NULL, NULL, 'GIg', 'qq'),
(2, 'Other Person', 'T', NULL, NULL, 'BQgJgJJQ', 'oxa', NULL, ''),
(3, 'Downey Jr., Robert', 'BBBF', NULL, 'OE', 'IK', 'ZZTTZTTZZZtZt', 'SSCY', 'gqgn');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'k', '', 'else'),
(2, 'Voice Character', 'nil', NULL, 'ZZZ', '000XX00', 'eey'),
(3, 'Other Character', NULL, NULL, '0', 'V4VV', 'undefined');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'bss', NULL, 2, 2005, NULL, NULL, 63, NULL, 122, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'then', '', NULL, NULL, NULL, 'ZZZQQtYZQZZZYZQ'),
(2, 2, ' 2', 'QHfQQvQHv', 'vKVVVKK', NULL, NULL, NULL),
(3, 2, '236', NULL, NULL, NULL, 'uM3', 'IbubuuqIbbbIbIIqqIbqu');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Q    _QS  _Q _ ', NULL, 2, 4758, 'GH', NULL, NULL, NULL, '-Infinity', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 1, 2, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'c'),
(2, 1, 2, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'Infinity', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 082/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'Inf', NULL, 3167, NULL, 'vX%XXGX', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '777TFFT', 8, NULL, NULL, NULL, 'GIg', 'qq'),
(2, 'Other Person', 'T', NULL, NULL, 'BQgJgJJQ', 'oxa', NULL, ''),
(3, 'Downey Jr., Robert', 'BBBF', NULL, 'OE', 'IK', 'ZZTTZTTZZZtZt', 'SSCY', 'gqgn');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'k', '', 'else'),
(2, 'Voice Character', 'nil', NULL, 'ZZZ', '000XX00', 'eey'),
(3, 'Other Character', NULL, NULL, '0', 'V4VV', 'undefined');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'bss', NULL, 2, 2005, NULL, NULL, 63, NULL, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '1', NULL, '', NULL, NULL, NULL),
(2, 2, 'ZZZQQtYZQZZZYZQ', NULL, NULL, NULL, 'QHfQQvQHv', 'vKVVVKK'),
(3, 2, '1', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '', NULL, 2, 1, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, 2, NULL, NULL, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', NULL),
(2, 1, 1, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 083/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'Inf', NULL, 3167, NULL, 'vX%XXGX', NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '777TFFT', 8, NULL, NULL, NULL, 'GIg', 'qq'),
(2, 'Other Person', 'T', NULL, NULL, 'BQgJgJJQ', 'oxa', NULL, ''),
(3, 'Downey Jr., Robert', 'BBBF', NULL, 'OE', 'IK', 'ZZTTZTTZZZtZt', 'SSCY', 'gqgn');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actor'),
(2, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, 'k', '', 'else'),
(2, 'Voice Character', 'nil', NULL, 'ZZZ', '000XX00', 'eey'),
(3, 'Other Character', NULL, NULL, '0', 'V4VV', 'undefined');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'bss', NULL, 2, 2005, NULL, NULL, 63, NULL, 122, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '777TFFT', '', NULL, NULL, NULL, 'ZZZQQtYZQZZZYZQ'),
(2, 2, ' 2', 'QHfQQvQHv', 'vKVVVKK', NULL, NULL, NULL),
(3, 2, '236', NULL, NULL, NULL, 'uM3', 'IbubuuqIbbbIbIIqqIbqu');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Q    _QS  _Q _ ', NULL, 2, 4758, 'GH', NULL, NULL, NULL, '-Infinity', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 3, 1, 2, '(uncredited)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'c'),
(2, 1, 2, 'USA', NULL),
(3, 1, 1, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'Infinity', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 084/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'TRUE', NULL, NULL, '', 'NULL', NULL),
(2, '', NULL, 3012, NULL, NULL, NULL),
(3, 'mZmZmmZZmZmZZmZZmZZm', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '00j');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'ycKmccyKmm', 'RJRRR', NULL, 'cc');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, '', 'Wf');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'then', 'BFBB', 2, 2012, 5071, 'C5YtYYtYv', NULL, 3990, 414, NULL, 'L');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '9', 'bdtdt', 'true', 'Infinity', NULL, NULL),
(2, 1, 'CCCCCCCCC', NULL, NULL, NULL, NULL, NULL),
(3, 1, 'K', 'R', NULL, 'ksKw''kwkkZswKZZK''w', '0', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '0', NULL, 1, 227, NULL, NULL, NULL, NULL, 'none', NULL),
(2, 1, 'vfbPvp', 'LLxL%imxmL%ixm%xximLxmimLx%m%', 2, NULL, 'cIIIco', NULL, 72, NULL, NULL, NULL),
(3, 1, '999999999999999999999999999999', 'CVV_PcPb', 1, 721, 'LPT1', 7046, 5476, 27, 'kn', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 2),
(2, 1, 1, NULL, '(uncredited)', 4868, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 3, 'bEb2Er1'),
(2, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'nBnBnBBBBnn');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', '0');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 3),
(2, 1, 1, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, '', NULL),
(2, 1, 1, 'INF', 'OWDDZ');
ROLLBACK;

-- ============================================================================
-- Generated database 085/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'TRUE', NULL, NULL, '', 'NULL', NULL),
(2, '', NULL, 3012, NULL, NULL, NULL),
(3, 'mZmZmmZZmZmZZmZZmZZm', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '00j');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'ycKmccyKmm', 'RJRRR', NULL, 'cc');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, '', 'Wf');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'then', 'BFBB', 2, 2012, 5071, 'C5YtYYtYv', NULL, 3990, 414, NULL, 'L');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '9', 'bdtdt', 'true', 'Infinity', NULL, NULL),
(2, 1, 'CCCCCCCCC', NULL, NULL, NULL, NULL, NULL),
(3, 1, 'K', 'R', NULL, 'ksKw''kwkkZswKZZK''w', '0', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '0', NULL, 1, 227, NULL, NULL, NULL, NULL, 'none', NULL),
(2, 1, 'vfbPvp', 'LLxL%imxmL%ixm%xximLxmimLx%m%', 2, NULL, 'cIIIco', NULL, 72, NULL, NULL, NULL),
(3, 1, '999999999999999999999999999999', 'CVV_PcPb', 1, 721, 'LPT1', 7046, 5476, 27, 'kn', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 2),
(2, 1, 1, NULL, '(uncredited)', 4868, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 3, 'bEb2Er1'),
(2, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'nBnBnBBBBnn');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', '0');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 3),
(2, 1, 1, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, '', NULL),
(2, 1, 2, 'INF', 'OWDDZ');
ROLLBACK;

-- ============================================================================
-- Generated database 086/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'TRUE', NULL, NULL, '', 'NULL', NULL),
(2, '', NULL, 3012, NULL, NULL, NULL),
(3, 'mZmZmmZZmZmZZmZZmZZm', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '00j');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'ycKmccyKmm', 'RJRRR', NULL, 'cc');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, '', 'Wf');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'then', NULL, 2, NULL, NULL, '1', NULL, 1, NULL, NULL, '1');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '1', NULL, NULL, NULL, NULL, NULL),
(2, 1, 'bdtdt', 'true', 'Infinity', NULL, NULL, NULL),
(3, 1, '1', NULL, NULL, NULL, NULL, NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '1', NULL, 2, NULL, 'ksKw''kwkkZswKZZK''w', NULL, 1, NULL, '', NULL),
(2, 1, '1', NULL, 2, NULL, NULL, NULL, NULL, 1, NULL, NULL),
(3, 1, 'vfbPvp', 'LLxL%imxmL%ixm%xximLxmimLx%m%', 2, NULL, 'cIIIco', NULL, 72, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', NULL, 1),
(2, 1, 1, 1, NULL, 7046, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 2, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 2, NULL),
(2, 1, 1, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', '1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2),
(2, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '1', 'nBnBnBBBBnn'),
(2, 1, 2, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 087/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'TRUE', NULL, NULL, '', 'NULL', NULL),
(2, '', NULL, 3012, NULL, NULL, NULL),
(3, 'mZmZmmZZmZmZZmZZmZZm', '[us]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'distributors'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '00j');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'follows'),
(3, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'ycKmccyKmm', 'RJRRR', NULL, 'cc');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, NULL, NULL, '', 'Wf');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'then', 'TRUE', 2, 2012, 5071, 'C5YtYYtYv', NULL, 3990, 414, NULL, 'L');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, '9', 'bdtdt', 'true', 'Infinity', NULL, NULL),
(2, 1, 'CCCCCCCCC', NULL, NULL, NULL, NULL, NULL),
(3, 1, 'K', 'R', NULL, 'ksKw''kwkkZswKZZK''w', '0', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, '0', NULL, 1, 227, NULL, NULL, NULL, NULL, 'none', NULL),
(2, 1, 'vfbPvp', 'LLxL%imxmL%ixm%xximLxmimLx%m%', 2, NULL, 'cIIIco', NULL, 72, NULL, NULL, NULL),
(3, 1, '999999999999999999999999999999', 'CVV_PcPb', 1, 721, 'LPT1', 7046, 5476, 27, 'kn', NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, NULL, NULL, 2),
(2, 1, 1, NULL, '(uncredited)', 4868, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 3, 'bEb2Er1'),
(2, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 2, 'USA', 'nBnBnBBBBnn');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 2, '4.0', '0');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 3),
(2, 1, 1, 3);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 2, '', NULL),
(2, 1, 1, 'INF', 'OWDDZ');
ROLLBACK;

-- ============================================================================
-- Generated database 088/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'NaN', NULL, NULL, '', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '__dict__');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 5, '__dict__', '', 'COM1', NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1365, NULL, NULL, ' '),
(2, 'Voice Character', 'z', 8, NULL, 'None', 'Wa');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-2', NULL, 1, 2010, 63, 'vo9oyAovofvA', NULL, 776, 1291, 'k', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, '0000000', '', 'Infinity'),
(2, 1, 'nil', '----', NULL, NULL, NULL, 'none');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Smh', 'Scunthorpe', 1, 329, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', 620, 1),
(2, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'Bulgaria', 'LPT1'),
(2, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '4.0', NULL),
(2, 1, 3, '4.0', '99''O9ZvZ');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'TRUE', 'false'),
(2, 1, 2, 'RGFRGJl_3', 'UFlUolUxk');
ROLLBACK;

-- ============================================================================
-- Generated database 089/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'NaN', NULL, NULL, '', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '__dict__');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 5, '__dict__', '', 'COM1', NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1365, '1', NULL, NULL),
(2, 'Voice Character', 'z', 8, NULL, 'None', 'Wa');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-2', NULL, 1, 2010, 63, 'vo9oyAovofvA', NULL, 776, 1291, 'k', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, '0000000', '', 'Infinity'),
(2, 1, 'nil', '----', NULL, NULL, NULL, 'none');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Smh', 'Scunthorpe', 1, 329, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', 620, 1),
(2, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'Bulgaria', 'LPT1'),
(2, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '4.0', NULL),
(2, 1, 3, '4.0', '99''O9ZvZ');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'TRUE', 'false'),
(2, 1, 2, 'RGFRGJl_3', 'UFlUolUxk');
ROLLBACK;

-- ============================================================================
-- Generated database 090/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'NaN', NULL, NULL, '', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '__dict__');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 5, '__dict__', '', 'COM1', NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1365, NULL, NULL, ' '),
(2, 'Voice Character', 'z', 8, NULL, 'None', 'Wa');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-2', NULL, 1, 2010, 1, 'vo9oyAovofvA', NULL, 776, 1291, 'k', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, '0000000', '', 'Infinity'),
(2, 1, 'nil', '----', NULL, NULL, NULL, 'none');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Smh', 'Scunthorpe', 1, 329, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', 620, 1),
(2, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'Bulgaria', 'LPT1'),
(2, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '4.0', NULL),
(2, 1, 3, '4.0', '99''O9ZvZ');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'TRUE', 'false'),
(2, 1, 2, 'RGFRGJl_3', 'UFlUolUxk');
ROLLBACK;

-- ============================================================================
-- Generated database 091/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'NaN', NULL, NULL, '', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '__dict__');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 5, '__dict__', '', 'COM1', NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1365, NULL, NULL, '----'),
(2, 'Voice Character', 'z', 8, NULL, 'None', 'Wa');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-2', NULL, 1, 2010, 63, 'vo9oyAovofvA', NULL, 776, 1291, 'k', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, '0000000', '', 'Infinity'),
(2, 1, 'nil', '----', NULL, NULL, NULL, 'none');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Smh', 'Scunthorpe', 1, 329, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', 620, 1),
(2, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'Bulgaria', 'LPT1'),
(2, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '4.0', NULL),
(2, 1, 3, '4.0', '99''O9ZvZ');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'TRUE', 'false'),
(2, 1, 2, 'RGFRGJl_3', 'UFlUolUxk');
ROLLBACK;

-- ============================================================================
-- Generated database 092/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete'),
(2, 'complete+verified'),
(3, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'NaN', NULL, NULL, '', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'distributors');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating'),
(2, 'rating'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '__dict__');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', NULL, 5, '__dict__', '', 'COM1', NULL, '');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, 1365, NULL, NULL, ' '),
(2, 'Voice Character', 'z', 8, NULL, 'None', 'Wa');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '-2', NULL, 1, 2010, 63, 'vo9oyAovofvA', NULL, 776, 1291, 'k', NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 1, 'undefined', NULL, NULL, '0000000', '', '----'),
(2, 1, 'nil', '----', NULL, NULL, NULL, 'none');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'Smh', 'Scunthorpe', 1, 329, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, NULL, '(voice)', 620, 1),
(2, 1, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, NULL, 1, 2);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 2, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 3, 'Bulgaria', 'LPT1'),
(2, 1, 2, 'USA', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 3, '4.0', NULL),
(2, 1, 3, '4.0', '99''O9ZvZ');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 1),
(2, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 2);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 3, 'TRUE', 'false'),
(2, 1, 2, 'RGFRGJl_3', 'UFlUolUxk');
ROLLBACK;

-- ============================================================================
-- Generated database 093/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[us]', 15, NULL, 'nil', NULL),
(2, '5UxAuu', '[de]', 108, NULL, 'Inf', NULL),
(3, 'm', '[ru]', 2088, 'y1U1c9c', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '00'),
(2, 'marvel-cinematic-universe', 'None'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'NULL', NULL, 'CSSCs', 'i-3', '', 'then', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, NULL, 'racc', 'hSchSjhhh');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, '0', 'Infinity', 'k'),
(2, 'Other Character', '__dict__', NULL, '', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'fT', 'DDADDxxDADDDxxxAxA', 2, NULL, NULL, NULL, 6786, 10000, 1, '', 'z');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'Scunthorpe', NULL, 'false', 'Infinity', 't', NULL),
(2, 1, 'v', NULL, NULL, 'NaN', 'UvXv', '999999999999999999999999999999'),
(3, 2, 'HfVfHVHHVwf', NULL, 'Infinity', NULL, '', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, 'BQpQQEpQBEQB-pEQypEGE-QQGB'),
(2, 1, '', 'AQ', 2, NULL, NULL, NULL, 1, NULL, NULL, NULL),
(3, 1, 'TRUE', NULL, 3, 1, NULL, NULL, NULL, NULL, 'eqOAOOdAOd', 'Cgi');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 2, '(voice)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 2, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '8.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'NULL', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 094/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[us]', 15, NULL, 'nil', NULL),
(2, '5UxAuu', '[de]', 108, NULL, 'Inf', NULL),
(3, 'm', '[ru]', 2088, 'y1U1c9c', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '00'),
(2, 'marvel-cinematic-universe', 'None'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'NULL', NULL, 'CSSCs', 'i-3', NULL, '', NULL),
(2, 'Other Person', NULL, NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Other Character', NULL, 1, NULL, NULL, NULL),
(2, 'Other Character', NULL, 1, NULL, 'Infinity', 'k');
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, '1', '__dict__', 2, 2006, NULL, NULL, NULL, 1, NULL, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '1', NULL, '1', NULL, '', 'z'),
(2, 2, 'Scunthorpe', NULL, 'false', 'Infinity', 't', NULL),
(3, 1, 'v', NULL, NULL, 'NaN', 'UvXv', '999999999999999999999999999999');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'HfVfHVHHVwf', NULL, 2, NULL, NULL, NULL, NULL, 1, '', NULL),
(2, 1, '1', NULL, 2, NULL, NULL, NULL, NULL, 1, NULL, NULL),
(3, 1, '', NULL, 2, NULL, NULL, NULL, NULL, 1, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 2, 2, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', '1');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', '1'),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 1),
(3, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, '1', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 095/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[us]', 15, NULL, 'nil', NULL),
(2, '5UxAuu', '[de]', 108, NULL, 'Inf', NULL),
(3, 'm', '[ru]', 2088, 'y1U1c9c', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '00'),
(2, 'marvel-cinematic-universe', 'None'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'NULL', NULL, 'CSSCs', 'i-3', '', 'then', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, NULL, 'racc', 'hSchSjhhh');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, '0', 'Infinity', 'k'),
(2, 'Other Character', 'k', NULL, '', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'fT', 'DDADDxxDADDDxxxAxA', 2, NULL, NULL, NULL, 6786, 10000, 1, '', 'z');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'Scunthorpe', NULL, 'false', 'Infinity', 't', NULL),
(2, 1, 'v', NULL, NULL, 'NaN', 'UvXv', '999999999999999999999999999999'),
(3, 2, 'HfVfHVHHVwf', NULL, 'Infinity', NULL, '', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, 'BQpQQEpQBEQB-pEQypEGE-QQGB'),
(2, 1, '', 'AQ', 2, NULL, NULL, NULL, 1, NULL, NULL, NULL),
(3, 1, 'TRUE', NULL, 3, 1, NULL, NULL, NULL, NULL, 'eqOAOOdAOd', 'Cgi');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 2, '(voice)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 2, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '8.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'NULL', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 096/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete+verified');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, '', '[us]', 15, NULL, 'nil', NULL),
(2, '5UxAuu', '[de]', 108, NULL, 'Inf', NULL),
(3, 'm', '[ru]', 2088, 'y1U1c9c', NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '00'),
(2, 'marvel-cinematic-universe', 'None'),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode'),
(3, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'NULL', NULL, 'CSSCs', 'i-3', '', 'then', NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, NULL, 'racc', 'hSchSjhhh');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', NULL, NULL, '0', 'Infinity', 'k'),
(2, 'Other Character', '__dict__', NULL, '', NULL, NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'fT', 'DDADDxxDADDDxxxAxA', 2, NULL, NULL, NULL, 6786, 1, 1, '', 'z');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, 'Scunthorpe', NULL, 'false', 'Infinity', 't', NULL),
(2, 1, 'v', NULL, NULL, 'NaN', 'UvXv', '999999999999999999999999999999'),
(3, 2, 'HfVfHVHHVwf', NULL, 'Infinity', NULL, '', '');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'True', NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, 'BQpQQEpQBEQB-pEQypEGE-QQGB'),
(2, 1, '', 'AQ', 2, NULL, NULL, NULL, 1, NULL, NULL, NULL),
(3, 1, 'TRUE', NULL, 3, 1, NULL, NULL, NULL, NULL, 'eqOAOOdAOd', 'Cgi');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 2, '(voice)', NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 3, 2, '');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '8.0', NULL),
(2, 1, 1, '8.0', NULL),
(3, 1, 1, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 1, 3),
(3, 1, 1);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 2, 1, 'NULL', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 097/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'WAAA', NULL, 1430, '0', 'WUUVCU', 'OB');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'NUL', 3899, NULL, NULL, NULL, 'QsQn656ss5nnnZQ5s66ZZ', '__proto__'),
(2, 'Downey Jr., Robert', NULL, 381, 'a''a''''''''Za''1', NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'else', NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'dNFNdPFFn', '', 1, 2009, 281, NULL, NULL, NULL, NULL, NULL, 'T4Z1_4Z');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '4''a4a4aaa', 'oo', 'True', NULL, 'LPT1', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'c', 'YUYrU5', 1, NULL, 'i9O', NULL, NULL, 336, NULL, NULL),
(2, 1, 'n', NULL, 2, NULL, 'wzywOw', 4526, NULL, 207, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, NULL, 1),
(2, 2, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, 'else');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'else'),
(2, 1, 1, 'USA', ''),
(3, 1, 1, 'USA', 'NIL');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', '__proto__'),
(2, 1, 1, '4.0', 'IIIIIIIIIIII');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '', NULL),
(2, 1, 1, 'Gq', NULL),
(3, 2, 1, 'True', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 098/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'WAAA', NULL, 1430, '0', 'WUUVCU', 'OB');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references'),
(2, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', 'NUL', 3899, NULL, NULL, NULL, 'QsQn656ss5nnnZQ5s66ZZ', 'wzywOw'),
(2, 'Downey Jr., Robert', NULL, 381, 'a''a''''''''Za''1', NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'else', NULL, NULL, '', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'dNFNdPFFn', '', 1, 2009, 281, NULL, NULL, NULL, NULL, NULL, 'T4Z1_4Z');
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 2, '4''a4a4aaa', 'oo', 'True', NULL, 'LPT1', NULL);
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 1, 'c', 'YUYrU5', 1, NULL, 'i9O', NULL, NULL, 336, NULL, NULL),
(2, 1, 'n', NULL, 2, NULL, 'wzywOw', 4526, NULL, 207, NULL, NULL);
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 2, 1, NULL, NULL, NULL, 1),
(2, 2, 1, NULL, NULL, NULL, 1);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, NULL, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, 'else');
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'Bulgaria', 'else'),
(2, 1, 1, 'USA', ''),
(3, 1, 1, 'USA', 'NIL');
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 1, 1, '4.0', '__proto__'),
(2, 1, 1, '4.0', 'IIIIIIIIIIII');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, '', NULL),
(2, 1, 1, 'Gq', NULL),
(3, 2, 1, 'True', NULL);
ROLLBACK;

-- ============================================================================
-- Generated database 099/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'b2LLLbL', NULL, NULL, '', '', ' 4%Qu');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '999999999999999999999999999999', 2903, NULL, 'then', '  _J_J JJ', NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '5UQ', '6 B8B BqU U8V 8U', 'False'),
(3, 'Other Person', 'LvL_ZZZLZv____LL', NULL, 'Q7Q', 'A', NULL, NULL, 'T');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'YvxY', 200, '', 'then', NULL),
(2, 'Other Character', NULL, 6631, NULL, 'NaN', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'bZWZjjWW', NULL, 2, 2009, 793, NULL, 8, NULL, NULL, 'mlmrNlNl', 'NaN'),
(2, 'null', 'uuuuuuuuu', 2, 2012, NULL, '7G', NULL, NULL, 43, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'b ', 'jj', 'bbb', NULL, NULL, '3lr3r');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, 'kZ''''kiZx', 'D0FeD0YN00Y3F', 1, NULL, NULL, NULL, NULL, NULL, 'f', 'INF');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, '(voice) (uncredited)', 292, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 2, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 2, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'mdTd0', 'if');
ROLLBACK;

-- ============================================================================
-- Generated database 100/100
-- ============================================================================
BEGIN TRANSACTION;
CREATE TABLE comp_cast_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO comp_cast_type VALUES
(1, 'complete');
CREATE TABLE company_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  country_code VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  name_pcode_sf VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO company_name VALUES
(1, 'b2LLLbL', NULL, NULL, '', '', ' 4%Qu');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'distributors'),
(2, 'production companies'),
(3, 'production companies');
CREATE TABLE info_type (
  id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO info_type VALUES
(1, 'countries'),
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'marvel-cinematic-universe', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'references');
CREATE TABLE name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  gender VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO name VALUES
(1, 'Other Person', '999999999999999999999999999999', 2903, NULL, 'then', '  _J_J JJ', NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, '5UQ', '6 B8B BqU U8V 8U', 'False'),
(3, 'Other Person', 'LvL_ZZZLZv____LL', NULL, 'Q7Q', 'A', NULL, NULL, 'T');
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
(2, 'actor');
CREATE TABLE char_name (
  id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  imdb_id BIGINT,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO char_name VALUES
(1, 'Voice Character', 'YvxY', 200, '', 'then', NULL),
(2, 'Other Character', NULL, 6631, NULL, 'NaN', NULL);
CREATE TABLE title (
  id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  imdb_id BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  series_years VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO title VALUES
(1, 'bZWZjjWW', NULL, 2, 2009, 793, NULL, 8, NULL, NULL, 'mlmrNlNl', 'NaN'),
(2, 'null', 'uuuuuuuuu', 2, 2012, NULL, '7G', NULL, NULL, 43, NULL, NULL);
CREATE TABLE aka_name (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  name VARCHAR NOT NULL,
  imdb_index VARCHAR,
  name_pcode_cf VARCHAR,
  name_pcode_nf VARCHAR,
  surname_pcode VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id)
);
INSERT INTO aka_name VALUES
(1, 3, 'b ', 'jj', 'bbb', NULL, NULL, '3lr3r');
CREATE TABLE aka_title (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  imdb_index VARCHAR,
  kind_id BIGINT NOT NULL,
  production_year BIGINT,
  phonetic_code VARCHAR,
  episode_of_id BIGINT,
  season_nr BIGINT,
  episode_nr BIGINT,
  note VARCHAR,
  md5sum VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (kind_id) REFERENCES kind_type(id)
);
INSERT INTO aka_title VALUES
(1, 2, '6 B8B BqU U8V 8U', 'D0FeD0YN00Y3F', 1, NULL, NULL, NULL, NULL, NULL, 'f', 'INF');
CREATE TABLE cast_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  person_role_id BIGINT,
  note VARCHAR,
  nr_order BIGINT,
  role_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (person_role_id) REFERENCES char_name(id),
  FOREIGN KEY (role_id) REFERENCES role_type(id)
);
INSERT INTO cast_info VALUES
(1, 1, 1, 1, '(voice) (uncredited)', 292, 2);
CREATE TABLE complete_cast (
  id BIGINT NOT NULL,
  movie_id BIGINT,
  subject_id BIGINT NOT NULL,
  status_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (subject_id) REFERENCES comp_cast_type(id),
  FOREIGN KEY (status_id) REFERENCES comp_cast_type(id)
);
INSERT INTO complete_cast VALUES
(1, 2, 1, 1);
CREATE TABLE movie_companies (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  company_id BIGINT NOT NULL,
  company_type_id BIGINT NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (company_id) REFERENCES company_name(id),
  FOREIGN KEY (company_type_id) REFERENCES company_type(id)
);
INSERT INTO movie_companies VALUES
(1, 1, 1, 1, NULL),
(2, 1, 1, 3, NULL);
CREATE TABLE movie_info (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info VALUES
(1, 1, 1, 'USA', NULL),
(2, 2, 3, 'Bulgaria', NULL);
CREATE TABLE movie_info_idx (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO movie_info_idx VALUES
(1, 2, 3, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 2),
(2, 2, 3);
CREATE TABLE movie_link (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  linked_movie_id BIGINT NOT NULL,
  link_type_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (linked_movie_id) REFERENCES title(id),
  FOREIGN KEY (link_type_id) REFERENCES link_type(id)
);
INSERT INTO movie_link VALUES
(1, 1, 1, 1),
(2, 2, 1, 1);
CREATE TABLE person_info (
  id BIGINT NOT NULL,
  person_id BIGINT NOT NULL,
  info_type_id BIGINT NOT NULL,
  info VARCHAR NOT NULL,
  note VARCHAR,
  PRIMARY KEY (id),
  FOREIGN KEY (person_id) REFERENCES name(id),
  FOREIGN KEY (info_type_id) REFERENCES info_type(id)
);
INSERT INTO person_info VALUES
(1, 1, 1, 'mdTd0', 'if');
ROLLBACK;

