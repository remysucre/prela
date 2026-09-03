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
(1, 'YYYY', NULL, 511, NULL, 'iIiiiIIIiII', NULL),
(2, '', '[ru]', NULL, NULL, NULL, '__proto__'),
(3, 'dd%K', NULL, 1023, NULL, NULL, NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', '');
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
(1, 'Other Person', NULL, 4938, NULL, NULL, NULL, NULL, '00'),
(2, 'Other Person', NULL, 1446, NULL, 'E', NULL, NULL, '999999999999999999999999999999');
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
(1, 'Other Character', NULL, NULL, NULL, NULL, 'pWW'),
(2, 'Voice Character', 'null', NULL, '', 'Infinity', 'none'),
(3, 'Voice Character', 'None', NULL, 'NIL', NULL, NULL);
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
(1, '__proto__', NULL, 1, NULL, NULL, NULL, 1, 16, NULL, NULL, 'LLLLLLLL');
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
(1, 1, 'null', NULL, NULL, NULL, NULL, NULL);
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
(1, 1, '91311999919', NULL, 1, NULL, 'UEnnn', NULL, NULL, 512, NULL, 'oLoLssoWo'),
(2, 1, 'hQQ', 'Fs QYFii', 1, NULL, NULL, 3401, 1, 793, 'Cb', NULL),
(3, 1, 'pp', NULL, 1, 31, NULL, 4096, 5332, NULL, NULL, 'Inf');
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
(1, 2, 1, NULL, '(voice) (uncredited)', 919, 2),
(2, 2, 1, NULL, '(voice)', 912, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', NULL, 1);
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
(2, 1, 1, 1),
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
(1, 1, 3, 1, NULL);
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
(1, 1, 1, 'Bulgaria', 'M6');
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
(2, 1, 2, '8.0', 'l  h');
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
(1, 1, 1, 2),
(2, 1, 1, 3),
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
(1, 1, 1, '-Infinity', 'KeR'),
(2, 2, 2, '', '3'),
(3, 2, 2, '', NULL);
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
(1, '_viy2MM_uM', NULL, 745, NULL, NULL, NULL),
(2, 'uZGnG', '[ru]', NULL, NULL, NULL, NULL),
(3, 'aaY''y-aY-yxYya-', NULL, 452, 'true', 'EH', 'vmvvwo');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies');
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
(1, 'marvel-cinematic-universe', '');
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
(1, 'Downey Jr., Robert', NULL, 580, 'COM1', NULL, 'None', NULL, NULL),
(2, 'Other Person', NULL, 150, NULL, '', NULL, NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, NULL, NULL, NULL, 'nil');
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
(1, 'Other Character', 'ea', 1376, NULL, '__proto__', 'TRUE');
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
(1, '22', NULL, 3, NULL, NULL, 'Kss', 6551, NULL, NULL, NULL, '');
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
(1, 2, 'if', '4ZlWV4qW2P', NULL, '6634C''i6ii6', NULL, '');
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
(1, 1, 'AAAAAAAAAAAAAAAAAAAAA', '''''''VV''V''''V''''''', 2, 3285, 'sssssssssssssssss', 931, NULL, NULL, '225TUTdb5U255U5Tdwbd3v3', ''),
(2, 1, 'wwwwwwwww', NULL, 3, NULL, 'U_U', NULL, NULL, 5174, NULL, NULL),
(3, 1, '555', 'XBYYu_XwxyuBuZZXu', 1, NULL, NULL, 530, 250, 336, '', NULL);
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
(1, 1, 2, 1, NULL),
(2, 1, 3, 2, '');
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
(1, 1, 2, 'Bulgaria', 'eQe%1c');
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
(1, 1, 1, '', NULL);
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
(1, 'null', '[us]', NULL, '', NULL, 'nil');
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', NULL),
(3, 'hero-sequel', NULL);
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
(1, 'Other Person', NULL, NULL, NULL, '', 'n', 'false', NULL),
(2, 'Other Person', 'bbbvRb55Rb5vRR', 31, NULL, '7H7ddx7xd7xHHxuKdm', NULL, 'Rd', NULL);
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
(1, 'Voice Character', NULL, 6256, 'undefined', 'CYYCtYCCCtCYCtCCYC', 'BBByEE'),
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
(1, '__dict__', 'if', 1, NULL, 6690, NULL, 1286, 0, NULL, NULL, NULL),
(2, '4TW66aa''', NULL, 2, NULL, 10, NULL, 2559, 2976, NULL, 'lllllllll', 'BBBBBBB');
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
(1, 1, 'r3', NULL, NULL, 'INF', 'ns', NULL),
(2, 1, 'R CR  s', '-Qk8kQ', NULL, NULL, NULL, NULL);
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
(1, 1, 'True', '__dict__', 1, NULL, 'Infinity', NULL, 5651, 755, NULL, NULL),
(2, 1, 'False', NULL, 1, NULL, NULL, 110, NULL, NULL, NULL, 'JEVeTJ');
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
(1, 2, 2, 1, NULL, 512, 1);
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
(1, 2, 2, 3);
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
(1, 2, 1, 1, '');
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
(1, 1, 1, 'USA', ''),
(2, 1, 1, 'USA', NULL);
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
(1, 1, 1, '4.0', 'o-u6ooB6'),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '4.0', 'Q5');
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
(1, 2, 2, 2),
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
(1, 1, 1, '1KPNWDDKQKDDQP9', '7a7BgL1L1');
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
(1, 'NULL', '[ru]', NULL, NULL, NULL, NULL);
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
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
(1, 'Downey Jr., Robert', NULL, 6788, NULL, NULL, 'ofKqoZI', 'kW', 'undefined'),
(2, 'Other Person', 'w', NULL, NULL, NULL, NULL, '%Y''i444', 'undefined');
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
(1, 'Voice Character', 'VVV44V4', 234, NULL, NULL, '__proto__'),
(2, 'Other Character', NULL, NULL, NULL, 'tt', 'pypyy');
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
(1, 'INF', NULL, 1, 2005, 387, 'MeC-M-MQ', NULL, NULL, 0, 'dkb', ''),
(2, 'y', 'HHHHHHHHHHHHHHHHHHHHHHHHHHH', 1, NULL, 3891, 'T', NULL, NULL, NULL, NULL, '0');
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
(1, 2, 'Zcc-7-', 'VZOZmO', NULL, 'lllll', NULL, NULL),
(2, 2, 'bta', 'fuwu8ffRff', 'null', 'msVs', NULL, NULL);
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
(1, 2, 'if', NULL, 1, 306, NULL, 2327, 2047, NULL, 'none', 'cxt'),
(2, 2, '5ZZZsiXkUZkik', 'SSEEHS', 1, 2590, NULL, NULL, NULL, NULL, 'h', NULL);
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
(1, 1, 1, 2, NULL, 402, 2),
(2, 2, 2, 1, NULL, 8, 1);
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
(2, 2, 1, 1);
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
(1, 1, 1, 1, 'PPPPPPPPPPPP');
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
(1, 1, 1, 'Bulgaria', '1e100');
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
(1, 1, 1, '4.0', 'rrr6%I');
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
(1, 1, 1, 'vR2', 'ttGxGjG');
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
(1, '22qS1v', NULL, NULL, '', NULL, '44282282484842');
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
(1, 'rating'),
(2, 'rating'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'if');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie'),
(3, 'movie');
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
(1, 'Other Person', '''6e9H9', 710, NULL, '44444444', '888888888', 'rOjO', '00'),
(2, 'Downey Jr., Robert', NULL, 136, NULL, 'null', 'NIL', NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Other Character', 'Scunthorpe', 999, NULL, 'NPTDDPemeDmPeeP', 'KyKy'),
(2, 'Other Character', 'fRLtuLLLRIIif', NULL, NULL, '9q99', 'NULL'),
(3, 'Voice Character', NULL, 290, NULL, 'tyt', NULL);
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
(1, 'INF', 'none', 3, NULL, NULL, NULL, 6642, 1665, 16, NULL, NULL);
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
(1, 2, 'False', NULL, NULL, 'USnShnnnpU7', 'g2', '        ');
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
(1, 1, '3L3LIGL LLGy  L3LI''LI8'' ', NULL, 3, 50, NULL, 189, NULL, 1497, NULL, '%XXXXB'),
(2, 1, 'meemmEg%e', '', 2, NULL, 'xIIPII', NULL, NULL, NULL, NULL, NULL),
(3, 1, 'Inf', 'eueeue', 2, 6205, 'Infinity', NULL, 1024, NULL, NULL, 'QQ');
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
(1, 2, 1, NULL, NULL, 189, 1),
(2, 1, 1, 3, NULL, 172, 3),
(3, 1, 1, NULL, NULL, NULL, 2);
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
(2, 1, 2, 2);
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
(2, 1, 1, 3, '');
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
(2, 1, 2, 'USA', '__proto__'),
(3, 1, 1, 'Bulgaria', 'BwwBwBwBwwBwwww');
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
(1, 1, 2, '8.0', 'nil'),
(2, 1, 2, '4.0', '__dict__'),
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
(2, 1, 1, 1),
(3, 1, 1, 3);
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
(1, 1, 2, 'O4GOE4OEOQQO4Ecc4cQ4EGEOUOEQ4cQE', NULL);
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
(1, 'PPVP', NULL, NULL, 'r', NULL, 'NUL'),
(2, 'j', '[ru]', 9090, 'true', '-o-b', 'k'),
(3, '', NULL, NULL, ' 6E f6', 'g', NULL);
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'none');
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
(1, 'Downey Jr., Robert', NULL, 11, '355', 'P7JPPJ', NULL, 'nil', NULL),
(2, 'Downey Jr., Robert', 'oLLLo', NULL, NULL, NULL, 'NaN', 'NIL', 'y '),
(3, 'Downey Jr., Robert', 'K08b', NULL, 'YYbZ', 'WlWvrdvvWFFrlw', 'WT', 'b3g3gb  3 3b33gbgb g', NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Voice Character', NULL, NULL, 'NaN', '', 'q'),
(2, 'Voice Character', NULL, 127, NULL, NULL, 'nJn');
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
(1, 'NUL', 'LPT1', 3, NULL, 8192, '''kG', NULL, 995, 8192, NULL, NULL),
(2, 'LPT1', NULL, 3, NULL, NULL, 'dCdC', 127, NULL, 271, 'Inf', '00');
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
(1, 3, '-Infinity', NULL, NULL, 'PddP   ', '66', NULL);
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
(1, 1, '', '5%kWuk%', 1, 0, NULL, 8192, NULL, NULL, NULL, NULL),
(2, 1, 'bsb', NULL, 2, 32, NULL, NULL, 786, NULL, NULL, 'Inf');
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
(1, 1, 2, 1, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, NULL, 3);
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
(1, 1, 2, 'Bulgaria', 'Inf'),
(2, 2, 2, 'Bulgaria', NULL);
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
(1, 1, 1, '4.0', 'ooosoos4444oosos4ss4o4sso4o44ss'),
(2, 1, 1, '4.0', 'AGsGAvssQvs');
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
(1, 1, 2, 2);
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
(1, 2, 2, 'false', 'false');
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
(1, 'none', NULL, NULL, '5', NULL, '');
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
(1, 'countries'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'INF'),
(2, 'hero-sequel', NULL),
(3, 'character-name-in-title', NULL);
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie'),
(3, 'movie');
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
(1, 'Downey Jr., Robert', 'llllll', NULL, 'K', NULL, NULL, 'rz5z55ttZt', NULL);
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
(1, 'Voice Character', 'FALSE', NULL, NULL, NULL, 'NUL');
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
(1, 'ejejejjP', 'K', 3, NULL, 8191, '', NULL, 20, 63, NULL, NULL),
(2, 'iia22iQa2i2', NULL, 3, 2012, 106, 'Vq', 4095, 64, 509, NULL, NULL),
(3, 'FALSE', NULL, 3, 2012, NULL, NULL, 2047, 31, NULL, 'D6fc66c', '''PPpvP''''vI1v1vvpv1');
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
(1, 1, '2222', 'NNHtMtP', NULL, '-Infinity', 'FF', 'gk'),
(2, 1, '', 'Scunthorpe', 'KKK', 'false', NULL, '666');
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
(1, 2, '999999999999999999999999999999', NULL, 1, NULL, 'yyyyByB', NULL, 145, 15, '999999999999999999999999999999', NULL);
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
(1, 1, 1, NULL, '(voice) (uncredited)', NULL, 1);
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
(1, NULL, 3, 3),
(2, NULL, 1, 3),
(3, 3, 1, 1);
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
(1, 1, 1, 2, 'Inf');
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
(1, 3, 2, 'USA', NULL),
(2, 1, 1, 'Bulgaria', NULL),
(3, 2, 2, 'Bulgaria', 'NIL');
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
(1, 3, 2, '8.0', '0D'),
(2, 3, 2, '4.0', 'RRYdrHYHHHrR'),
(3, 3, 2, '8.0', NULL);
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
(1, 1, 3, 2),
(2, 1, 2, 2),
(3, 2, 1, 2);
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
(1, 1, 2, '%', NULL),
(2, 1, 2, '', 'FC');
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
(1, 'True', NULL, 224, 'COM1', '__proto__', NULL),
(2, '00', '[de]', 207, NULL, NULL, NULL);
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
(2, 'marvel-cinematic-universe', '7Ot3vOO7I3BvvBU37OOB773OUIOB');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'movie'),
(3, 'episode');
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
(1, 'Downey Jr., Robert', NULL, 587, 'NaN', 'XrkrXk', NULL, NULL, NULL);
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
(1, 'Other Character', NULL, 103, 'kQQnkQQnnQQ', NULL, NULL),
(2, 'Voice Character', NULL, 649, '0C3200S00330%C%CHCSF%C%', NULL, NULL),
(3, 'Other Character', NULL, NULL, 'null', '3333', NULL);
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
(1, '00', NULL, 3, 2005, 38, '', NULL, 233, 2862, NULL, NULL);
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
(1, 1, 'OLOrrELErLOOEr', 'NUL', '', 'gggggggggg', 'XXX----', NULL);
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
(1, 1, '9t', '___', 1, NULL, 'false', NULL, NULL, NULL, 'FBdLFddVB', NULL),
(2, 1, '2', NULL, 2, 2048, 'true', NULL, NULL, 249, '  ', NULL),
(3, 1, '', 'lOlOlllllxOxOOllxOxOOxxxlx', 3, NULL, 'jjjjjj', NULL, NULL, NULL, NULL, 'a');
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
(1, 1, 1, 2, NULL, 1023, 2),
(2, 1, 1, NULL, '(voice) (uncredited)', NULL, 3);
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
(1, 1, 1, 3),
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
(1, 1, 2, 1, NULL),
(2, 1, 2, 3, 'if'),
(3, 1, 1, 3, '73');
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
(1, 1, 2, 'USA', 'ZG3WA'),
(2, 1, 3, 'USA', NULL),
(3, 1, 2, 'Bulgaria', '____v');
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
(1, 1, 3, '8.0', NULL),
(2, 1, 2, '4.0', NULL),
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
(1, 1, 2, 'undefined', NULL);
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
(1, 'complete'),
(2, 'complete'),
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
(1, 'then', NULL, 5566, 'True', 'then', 'True'),
(2, '', '[us]', 218, NULL, 'LPT1', '-Infinity'),
(3, '8', NULL, 4, NULL, 'LPT1', NULL);
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'dPOO-P-d'),
(2, 'character-name-in-title', 'u');
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
(1, 'references'),
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
(1, 'Other Person', NULL, NULL, NULL, NULL, 'True', NULL, '0'),
(2, 'Downey Jr., Robert', 'AA', NULL, 'BtVVVtBtBVtVttttVtVVVV', 'Inf', '', '8t', 'VVV');
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
(1, 'Voice Character', 'INF', NULL, NULL, '8888', ''),
(2, 'Other Character', 'GGRRf5R', 4, 'true', 'Infinity', NULL),
(3, 'Other Character', NULL, 1805, NULL, 'FALSE', NULL);
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
(1, 'hddhihdhhdd', 'None', 1, 2012, NULL, '', 2, 1023, NULL, 'EE', ''),
(2, 'K', 'false', 1, NULL, NULL, NULL, 7, 4712, 1084, 'Y', 'a558zGRza8cGGc'),
(3, '%sY', NULL, 1, NULL, NULL, 'M', 6, NULL, NULL, NULL, '0');
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
(1, 2, 'Inf', NULL, NULL, NULL, 'JJJJJJJJJJJ', '');
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
(1, 1, '''''''gYf''gY', NULL, 1, 2366, 'Z3Wd3d33dZZ3WZ33d', NULL, NULL, 287, 'u-V-LqVVLjL', 'wwwwwwwwwww'),
(2, 2, 'LZZLHLHHJH', 'vvvvvvvvvvvvvvv', 1, NULL, 'Inf', 63, 6, NULL, NULL, NULL);
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
(1, 1, 1, NULL, NULL, 1282, 2),
(2, 1, 3, 2, NULL, 4, 1);
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
(1, 2, 1, 3);
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
(1, 2, 1, 3, '0'),
(2, 3, 1, 1, '5TVW'),
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
(1, 1, 2, 'USA', NULL),
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
(1, 3, 1, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 2),
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
(1, 2, 2, 3),
(2, 3, 1, 2),
(3, 3, 1, 3);
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
(1, 1, 2, '0', NULL),
(2, 1, 2, 'OgGOSg%', NULL);
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
(1, 'complete'),
(2, 'complete'),
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
(1, 'then', NULL, 5566, 'True', 'then', 'True'),
(2, '', '[us]', 218, NULL, 'LPT1', '-Infinity'),
(3, '8', NULL, 4, NULL, 'LPT1', NULL);
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'dPOO-P-d'),
(2, 'character-name-in-title', 'u');
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
(1, 'references'),
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
(1, 'Other Person', NULL, NULL, NULL, NULL, 'True', NULL, '0'),
(2, 'Downey Jr., Robert', 'AA', NULL, 'BtVVVtBtBVtVttttVtVVVV', 'Inf', '', '8t', 'VVV');
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
(1, 'Voice Character', 'INF', NULL, NULL, '8888', ''),
(2, 'Other Character', 'GGRRf5R', 4, 'true', 'Infinity', NULL),
(3, 'Other Character', NULL, 1805, NULL, 'FALSE', NULL);
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
(1, 'hddhihdhhdd', 'None', 1, 2012, NULL, '', 2, 1023, NULL, 'EE', ''),
(2, 'K', 'false', 1, NULL, NULL, NULL, 7, 4712, 1084, 'Y', 'a558zGRza8cGGc'),
(3, '%sY', NULL, 1, NULL, NULL, 'M', NULL, NULL, NULL, NULL, NULL);
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
(1, 2, '0', NULL, NULL, NULL, NULL, NULL);
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
(1, 2, '', NULL, 1, 1, NULL, NULL, NULL, NULL, NULL, '1'),
(2, 2, '1', NULL, 1, NULL, 'u-V-LqVVLjL', 1, NULL, NULL, NULL, NULL);
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
(2, 2, 2, 2, '(voice)', NULL, 2);
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
(1, 2, 2, 2, NULL),
(2, 2, 1, 3, '1'),
(3, 2, 2, 1, '1');
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
(1, 3, 2, 'USA', NULL),
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
(1, 2, 2),
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
(1, 2, 2, 1),
(2, 2, 1, 2),
(3, 2, 2, 1);
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
(1, 2, 2, '1', NULL),
(2, 2, 2, '1', NULL);
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
(1, 'complete'),
(2, 'complete'),
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
(1, 'then', NULL, 5566, 'True', 'then', 'True'),
(2, '', '[us]', 218, NULL, 'LPT1', '-Infinity'),
(3, '8', NULL, 4, NULL, 'LPT1', NULL);
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'dPOO-P-d'),
(2, 'character-name-in-title', 'u');
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
(1, 'references'),
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
(1, 'Other Person', NULL, NULL, NULL, NULL, 'True', NULL, '0'),
(2, 'Downey Jr., Robert', 'AA', NULL, 'BtVVVtBtBVtVttttVtVVVV', 'Inf', '', '8t', 'VVV');
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
(1, 'Voice Character', 'INF', NULL, NULL, '8888', ''),
(2, 'Other Character', NULL, NULL, NULL, '1', NULL),
(3, 'Other Character', 'Infinity', NULL, NULL, NULL, '1');
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
(1, '1', NULL, 1, NULL, NULL, 'None', NULL, 2012, NULL, NULL, ''),
(2, '1', NULL, 1, NULL, 1, NULL, 1, NULL, 1, NULL, NULL),
(3, '1', NULL, 1, 2006, 4712, '1', NULL, NULL, 1, NULL, NULL);
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
(1, 2, '1', NULL, 'M', '1', NULL, NULL);
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
(1, 2, '0', NULL, 1, NULL, NULL, NULL, NULL, 1, NULL, ''),
(2, 1, '''''''gYf''gY', NULL, 1, 2366, 'Z3Wd3d33dZZ3WZ33d', NULL, NULL, 287, 'u-V-LqVVLjL', 'wwwwwwwwwww');
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
(1, 2, 2, NULL, '(voice)', NULL, 1),
(2, 2, 2, NULL, NULL, 63, 2);
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
(2, 2, 2, 2, NULL),
(3, 1, 3, 2, NULL);
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
(1, 2, 1, '4.0', '0');
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
(1, 2, 2, 2),
(2, 3, 1, 2),
(3, 2, 1, 2);
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
(2, 2, 2, '1', NULL);
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
(1, 'false', '[ru]', NULL, '6fefLvfvef', '', 'W'),
(2, 'llKlllKl', NULL, 32, NULL, '''eII', 'MMR88M8 88R'),
(3, 'w''wZZ', NULL, 15, NULL, NULL, '3i3ZbZZ');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'character-name-in-title', 'COM1');
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
(1, 'Downey Jr., Robert', NULL, 4, 'Infinity', 'sssoAAnS_tss', NULL, NULL, NULL);
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
(1, 'Voice Character', '6Tj88', NULL, NULL, 'undefined', NULL);
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
(1, 'Inf', NULL, 3, 2005, NULL, NULL, NULL, NULL, NULL, NULL, 'J_'),
(2, 'f6BB', '88PF', 3, NULL, NULL, '3ra-a3', 8191, 255, NULL, 'Infinity', '0'),
(3, 'J', '-----------', 2, 2012, NULL, NULL, NULL, NULL, NULL, 'm3GY', 'SkBGGGbb');
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
(1, 1, 'false', '''D', NULL, NULL, NULL, 'LLffLfLLfLLfLsLsfLsL'),
(2, 1, '0', 'KKK', 'JvJJ', '0', NULL, NULL),
(3, 1, '', NULL, NULL, NULL, 'y', NULL);
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
(1, 3, 'zfz8zAzfAAu', 'CYC''P', 1, NULL, NULL, NULL, NULL, 511, 'b', '3T323mL333sn2');
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
(1, 1, 2, NULL, '(uncredited)', 1023, 1),
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
(1, 1, 3, 2, 'NaN'),
(2, 2, 3, 1, NULL),
(3, 3, 1, 1, NULL);
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
(1, 3, 1, 'Bulgaria', 'DDeCTDeeTDT9'),
(2, 1, 1, 'USA', NULL);
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
(2, 2, 1, '8.0', NULL),
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
(1, 3, 3, 1),
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
(1, 1, 1, 'fQjFQvfDbvDQ', 'LPT1'),
(2, 1, 1, 'PjBssN', 'E');
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
(1, 'false', '[ru]', NULL, '6fefLvfvef', '', 'W'),
(2, 'llKlllKl', NULL, 32, NULL, '''eII', 'MMR88M8 88R'),
(3, 'w''wZZ', NULL, 15, NULL, NULL, '3i3ZbZZ');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'character-name-in-title', 'COM1');
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
(1, 'Downey Jr., Robert', NULL, 4, 'Infinity', 'false', NULL, NULL, NULL);
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
(1, 'Voice Character', '6Tj88', NULL, NULL, 'undefined', NULL);
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
(1, 'Inf', NULL, 3, 2005, NULL, NULL, NULL, NULL, NULL, NULL, 'J_'),
(2, 'f6BB', '88PF', 3, NULL, NULL, '3ra-a3', 8191, 255, NULL, 'Infinity', '0'),
(3, 'J', '-----------', 2, 2012, NULL, NULL, NULL, NULL, NULL, 'm3GY', 'SkBGGGbb');
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
(1, 1, 'false', '''D', NULL, NULL, NULL, 'LLffLfLLfLLfLsLsfLsL'),
(2, 1, '0', 'KKK', 'JvJJ', '0', NULL, NULL),
(3, 1, '', NULL, NULL, NULL, 'y', NULL);
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
(1, 3, 'zfz8zAzfAAu', 'CYC''P', 1, NULL, NULL, NULL, NULL, 511, 'b', '3T323mL333sn2');
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
(1, 1, 2, NULL, '(uncredited)', 1023, 1),
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
(1, 1, 3, 2, 'NaN'),
(2, 2, 3, 1, NULL),
(3, 3, 1, 1, NULL);
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
(1, 3, 1, 'Bulgaria', 'DDeCTDeeTDT9'),
(2, 1, 1, 'USA', NULL);
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
(2, 2, 1, '8.0', NULL),
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
(1, 3, 3, 1),
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
(1, 1, 1, 'fQjFQvfDbvDQ', 'LPT1'),
(2, 1, 1, 'PjBssN', 'E');
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
(1, 'false', '[ru]', NULL, '6fefLvfvef', '', 'W'),
(2, 'llKlllKl', NULL, 32, NULL, '''eII', 'MMR88M8 88R'),
(3, 'w''wZZ', NULL, 15, NULL, NULL, '3i3ZbZZ');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'character-name-in-title', 'COM1');
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
(1, 'Downey Jr., Robert', NULL, 4, 'Infinity', 'sssoAAnS_tss', NULL, NULL, NULL);
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
(1, 'Voice Character', '6Tj88', NULL, NULL, 'undefined', NULL);
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
(1, 'Inf', NULL, 3, 2005, NULL, NULL, NULL, NULL, NULL, NULL, 'J_'),
(2, 'f6BB', '88PF', 3, NULL, NULL, '3ra-a3', 8191, 255, NULL, 'Infinity', '0'),
(3, 'J', '-----------', 2, 2012, NULL, NULL, NULL, NULL, NULL, 'm3GY', 'SkBGGGbb');
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
(1, 1, 'false', '''D', NULL, NULL, NULL, 'LLffLfLLfLLfLsLsfLsL'),
(2, 1, '0', 'KKK', 'JvJJ', '0', NULL, NULL),
(3, 1, '', NULL, NULL, NULL, 'y', NULL);
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
(1, 3, 'zfz8zAzfAAu', 'CYC''P', 1, NULL, NULL, NULL, NULL, 511, 'b', '3T323mL333sn2');
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
(1, 1, 2, NULL, '(uncredited)', 1023, 1),
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
(1, 1, 3, 2, 'NaN'),
(2, 2, 3, 1, NULL),
(3, 3, 1, 1, NULL);
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
(1, 3, 1, 'Bulgaria', 'DDeCTDeeTDT9'),
(2, 1, 1, 'USA', NULL);
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
(2, 2, 1, '8.0', NULL),
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
(1, 3, 3, 1),
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
(1, 1, 1, 'fQjFQvfDbvDQ', 'CYC''P'),
(2, 1, 1, 'PjBssN', 'E');
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
(1, 'false', '[ru]', NULL, '6fefLvfvef', '', 'W'),
(2, 'llKlllKl', NULL, 32, NULL, '''eII', 'MMR88M8 88R'),
(3, 'w''wZZ', NULL, 15, NULL, NULL, '3i3ZbZZ');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'character-name-in-title', 'COM1');
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
(1, 'Downey Jr., Robert', NULL, 4, 'Infinity', 'sssoAAnS_tss', NULL, NULL, NULL);
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
(1, 'Voice Character', '''D', NULL, NULL, 'undefined', NULL);
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
(1, 'Inf', NULL, 3, 2005, NULL, NULL, NULL, NULL, NULL, NULL, 'J_'),
(2, 'f6BB', '88PF', 3, NULL, NULL, '3ra-a3', 8191, 255, NULL, 'Infinity', '0'),
(3, 'J', '-----------', 2, 2012, NULL, NULL, NULL, NULL, NULL, 'm3GY', 'SkBGGGbb');
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
(1, 1, 'false', '''D', NULL, NULL, NULL, 'LLffLfLLfLLfLsLsfLsL'),
(2, 1, '0', 'KKK', 'JvJJ', '0', NULL, NULL),
(3, 1, '', NULL, NULL, NULL, 'y', NULL);
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
(1, 3, 'zfz8zAzfAAu', 'CYC''P', 1, NULL, NULL, NULL, NULL, 511, 'b', '3T323mL333sn2');
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
(1, 1, 2, NULL, '(uncredited)', 1023, 1),
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
(1, 1, 3, 2, 'NaN'),
(2, 2, 3, 1, NULL),
(3, 3, 1, 1, NULL);
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
(1, 3, 1, 'Bulgaria', 'DDeCTDeeTDT9'),
(2, 1, 1, 'USA', NULL);
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
(2, 2, 1, '8.0', NULL),
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
(1, 3, 3, 1),
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
(1, 1, 1, 'fQjFQvfDbvDQ', 'LPT1'),
(2, 1, 1, 'PjBssN', 'E');
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
(1, 'false', '[ru]', NULL, '6fefLvfvef', '', 'W'),
(2, 'llKlllKl', NULL, 32, NULL, '''eII', 'MMR88M8 88R'),
(3, 'w''wZZ', NULL, 15, NULL, NULL, '3i3ZbZZ');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'marvel-cinematic-universe', NULL),
(3, 'character-name-in-title', 'COM1');
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
(1, 'Downey Jr., Robert', NULL, 4, 'Infinity', 'sssoAAnS_tss', NULL, NULL, NULL);
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
(1, 'Voice Character', '6Tj88', NULL, NULL, 'undefined', NULL);
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
(1, 'Inf', NULL, 3, 2005, NULL, NULL, NULL, NULL, NULL, NULL, 'J_'),
(2, 'f6BB', '88PF', 3, NULL, NULL, '3ra-a3', 8191, 255, NULL, 'Infinity', '0'),
(3, 'CYC''P', '-----------', 2, 2012, NULL, NULL, NULL, NULL, NULL, 'm3GY', 'SkBGGGbb');
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
(1, 1, 'false', '''D', NULL, NULL, NULL, 'LLffLfLLfLLfLsLsfLsL'),
(2, 1, '0', 'KKK', 'JvJJ', '0', NULL, NULL),
(3, 1, '', NULL, NULL, NULL, 'y', NULL);
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
(1, 3, 'zfz8zAzfAAu', 'CYC''P', 1, NULL, NULL, NULL, NULL, 511, 'b', '3T323mL333sn2');
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
(1, 1, 2, NULL, '(uncredited)', 1023, 1),
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
(1, 1, 3, 2, 'NaN'),
(2, 2, 3, 1, NULL),
(3, 3, 1, 1, NULL);
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
(1, 3, 1, 'Bulgaria', 'DDeCTDeeTDT9'),
(2, 1, 1, 'USA', NULL);
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
(2, 2, 1, '8.0', NULL),
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
(1, 3, 3, 1),
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
(1, 1, 1, 'fQjFQvfDbvDQ', 'LPT1'),
(2, 1, 1, 'PjBssN', 'E');
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
(1, 'fbfOr', '[us]', NULL, NULL, NULL, NULL),
(2, 'None', NULL, 641, '0', 'WAW---W-AW', NULL),
(3, 'None', '[us]', 2048, NULL, NULL, '');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies');
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
(1, 'marvel-cinematic-universe', 'n');
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
(1, 'Downey Jr., Robert', NULL, 7929, '9dQd', ' RU n Pu', 'COM1', 'HHHHHHHH', NULL),
(2, 'Other Person', NULL, 175, NULL, NULL, 'vZI5I', NULL, 'INF'),
(3, 'Downey Jr., Robert', 'R', NULL, '__proto__', 'cWWcWcW', 'FFFg3gr3', 'NUL', NULL);
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
(1, 'Other Character', 'false', NULL, 'k9kk9r', 'NUL', ''),
(2, 'Other Character', NULL, NULL, 'if', 'SSSS', 'NUL');
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
(1, '8r--', NULL, 1, 2009, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'g6Q6', NULL, 1, 2008, 5065, NULL, NULL, 9331, NULL, '6W6e', NULL);
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
(1, 3, 'w03B9g30O44B9Og', '  ed0b7d', NULL, NULL, NULL, 'C_nuh4_hhuh4b4b');
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
(1, 2, 'Infinity', NULL, 1, NULL, 'RX', NULL, NULL, NULL, NULL, NULL),
(2, 2, 'j9', NULL, 1, NULL, NULL, NULL, NULL, 0, '%''5%', '');
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
(1, 1, 1, 1, '(voice)', NULL, 3),
(2, 2, 2, 2, '(voice) (uncredited)', 712, 3);
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
(1, 1, 1, 1, 'l1RR1jRj11j'),
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
(1, 2, 1, 'Bulgaria', 'U-KweKeIK-A--'),
(2, 2, 2, 'USA', 'LPT1');
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
(1, 2, 1, '8.0', 'iVEYY'),
(2, 2, 3, '8.0', NULL);
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
(2, 2, 1),
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
(1, 2, 2, '''45U5nnU', 'OROdt'),
(2, 1, 1, '999999999999999999999999999999', '7p8'),
(3, 1, 1, '-Infinity', 'I7_iIi_ii_I');
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
(1, 'fbfOr', '[us]', NULL, NULL, NULL, NULL),
(2, 'None', NULL, 641, '0', 'WAW---W-AW', NULL),
(3, 'None', '[us]', 2048, NULL, NULL, '');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies');
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
(1, 'marvel-cinematic-universe', 'n');
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
(1, 'Downey Jr., Robert', NULL, 7929, '9dQd', ' RU n Pu', 'COM1', 'HHHHHHHH', NULL),
(2, 'Other Person', NULL, 175, NULL, NULL, 'vZI5I', NULL, 'INF'),
(3, 'Downey Jr., Robert', 'R', NULL, '__proto__', 'cWWcWcW', 'FFFg3gr3', 'NUL', NULL);
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
(1, 'Other Character', 'false', NULL, 'k9kk9r', 'NUL', ''),
(2, 'Other Character', NULL, NULL, 'if', 'SSSS', 'NUL');
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
(1, '8r--', NULL, 1, 2009, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'g6Q6', NULL, 1, 2008, 5065, NULL, NULL, 9331, NULL, '6W6e', NULL);
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
(1, 3, 'w03B9g30O44B9Og', '  ed0b7d', NULL, NULL, NULL, 'C_nuh4_hhuh4b4b');
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
(1, 2, 'Infinity', NULL, 1, NULL, 'RX', NULL, NULL, NULL, NULL, NULL),
(2, 2, 'j9', NULL, 1, NULL, NULL, NULL, NULL, 0, '%''5%', '');
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
(1, 1, 1, 1, '(voice)', NULL, 3),
(2, 2, 2, 2, '(voice) (uncredited)', 712, 3);
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
(1, 1, 1, 1, 'l1RR1jRj11j'),
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
(1, 2, 1, 'Bulgaria', 'U-KweKeIK-A--'),
(2, 2, 2, 'USA', 'LPT1');
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
(1, 2, 1, '8.0', 'iVEYY'),
(2, 2, 3, '8.0', NULL);
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
(2, 2, 1),
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
(1, 2, 2, '''45U5nnU', 'OROdt');
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
(1, 'fbfOr', '[us]', NULL, NULL, NULL, NULL),
(2, 'None', NULL, 641, '0', 'WAW---W-AW', NULL),
(3, 'None', '[us]', 2048, NULL, NULL, NULL);
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
(1, 'Other Person', NULL, NULL, NULL, NULL, '1', NULL, NULL),
(2, 'Other Person', NULL, NULL, 'COM1', 'HHHHHHHH', NULL, NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, 'vZI5I', NULL, 'INF', NULL);
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
(1, 'Other Character', '__proto__', 1, NULL, 'FFFg3gr3', 'NUL'),
(2, 'Other Character', NULL, NULL, NULL, NULL, 'false');
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
(1, '1', 'k9kk9r', 1, NULL, NULL, NULL, 1, NULL, NULL, NULL, 'if'),
(2, '1', NULL, 1, 2006, NULL, NULL, NULL, NULL, 2009, NULL, NULL);
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
(1, 2, '1', NULL, NULL, NULL, NULL, '1');
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
(1, 2, '1', '1', 1, NULL, NULL, NULL, NULL, NULL, NULL, '  ed0b7d'),
(2, 2, '1', 'C_nuh4_hhuh4b4b', 1, NULL, NULL, NULL, NULL, NULL, 'RX', NULL);
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
(2, 2, 2, NULL, NULL, NULL, 2);
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
(1, 1, 2, 1, NULL),
(2, 2, 2, 2, '1');
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
(2, 2, 1, 'USA', '1');
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
(2, 1, 1, '4.0', 'l1RR1jRj11j');
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
(1, 2, 2, '1', 'LPT1'),
(2, 2, 1, '1', NULL),
(3, 2, 2, '1', NULL);
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
(1, 'fbfOr', '[us]', NULL, NULL, NULL, NULL),
(2, 'None', NULL, 641, '0', 'WAW---W-AW', NULL),
(3, 'None', '[us]', NULL, NULL, NULL, NULL);
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
(2, 'countries'),
(3, 'rating');
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
(1, 'Other Person', NULL, NULL, NULL, NULL, NULL, '1', NULL),
(2, 'Other Person', ' RU n Pu', 1, NULL, 'HHHHHHHH', NULL, NULL, NULL),
(3, 'Other Person', NULL, NULL, NULL, 'vZI5I', NULL, 'INF', NULL);
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
(1, 'Other Character', '__proto__', 1, NULL, 'FFFg3gr3', 'NUL'),
(2, 'Other Character', NULL, NULL, NULL, NULL, 'false');
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
(1, '1', 'k9kk9r', 1, NULL, NULL, NULL, 1, NULL, NULL, NULL, 'if'),
(2, '1', NULL, 1, 2006, NULL, NULL, NULL, NULL, 2009, NULL, NULL);
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
(1, 2, '1', NULL, NULL, NULL, NULL, '1');
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
(1, 2, '1', '1', 1, NULL, NULL, NULL, NULL, NULL, NULL, '  ed0b7d'),
(2, 2, '1', 'C_nuh4_hhuh4b4b', 1, NULL, NULL, NULL, NULL, NULL, 'RX', NULL);
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
(2, 2, 2, NULL, NULL, NULL, 2);
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
(1, 1, 2, 1, NULL),
(2, 2, 2, 2, '1');
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
(2, 2, 1, 'USA', '1');
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
(2, 1, 1, '4.0', 'l1RR1jRj11j');
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
(1, 2, 2, '1', 'LPT1'),
(2, 2, 1, '1', NULL),
(3, 2, 2, '1', NULL);
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
(1, 'NUL', '[us]', NULL, NULL, NULL, NULL),
(2, 'None', NULL, 641, '0', 'WAW---W-AW', NULL),
(3, 'None', '[us]', 2048, NULL, NULL, '');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies');
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
(1, 'marvel-cinematic-universe', 'n');
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
(1, 'Downey Jr., Robert', NULL, 7929, '9dQd', ' RU n Pu', 'COM1', 'HHHHHHHH', NULL),
(2, 'Other Person', NULL, 175, NULL, NULL, 'vZI5I', NULL, 'INF'),
(3, 'Downey Jr., Robert', 'R', NULL, '__proto__', 'cWWcWcW', 'FFFg3gr3', 'NUL', NULL);
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
(1, 'Other Character', 'false', NULL, 'k9kk9r', 'NUL', ''),
(2, 'Other Character', NULL, NULL, 'if', 'SSSS', 'NUL');
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
(1, '8r--', NULL, 1, 2009, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'g6Q6', NULL, 1, 2008, 5065, NULL, NULL, 9331, NULL, '6W6e', NULL);
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
(1, 3, 'w03B9g30O44B9Og', '  ed0b7d', NULL, NULL, NULL, 'C_nuh4_hhuh4b4b');
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
(1, 2, 'Infinity', NULL, 1, NULL, 'RX', NULL, NULL, NULL, NULL, NULL),
(2, 2, 'j9', NULL, 1, NULL, NULL, NULL, NULL, 0, '%''5%', '');
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
(1, 1, 1, 1, '(voice)', NULL, 3),
(2, 2, 2, 2, '(voice) (uncredited)', 712, 3);
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
(1, 1, 1, 1, 'l1RR1jRj11j'),
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
(1, 2, 1, 'Bulgaria', 'U-KweKeIK-A--'),
(2, 2, 2, 'USA', 'LPT1');
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
(1, 2, 1, '8.0', 'iVEYY'),
(2, 2, 3, '8.0', NULL);
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
(2, 2, 1),
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
(1, 2, 2, '''45U5nnU', 'OROdt'),
(2, 1, 1, '999999999999999999999999999999', '7p8'),
(3, 1, 1, '-Infinity', 'I7_iIi_ii_I');
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
(1, '77gd7dg', NULL, NULL, NULL, NULL, NULL),
(2, 'pKKfHhjphpRKpjj', NULL, NULL, 'Scunthorpe', 'Scunthorpe', '9J%d999'),
(3, '__proto__', '[ru]', NULL, NULL, '2', 'b');
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
(1, 'marvel-cinematic-universe', '3'),
(2, 'marvel-cinematic-universe', 'vFTvT');
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
(1, 'Downey Jr., Robert', 'Jw', NULL, 'o', NULL, '''''N2z66O6-6', NULL, NULL);
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
(1, 'Voice Character', NULL, NULL, NULL, '0', NULL),
(2, 'Other Character', 'Mw', 5, '', NULL, '4');
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
(1, 'mUeUwmmeUUUmmeUeUUwwwe', 'f8fiSKG', 1, 2006, 512, NULL, 512, 128, NULL, NULL, 'Inf'),
(2, 'hhhh', NULL, 1, 2007, NULL, NULL, 697, 4369, 511, NULL, 'jj'),
(3, 'JgJJgg''''g', NULL, 1, 2006, NULL, NULL, 2919, 443, 127, NULL, NULL);
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
(1, 1, 'b', NULL, NULL, NULL, NULL, NULL),
(2, 1, '6rCr2', '5', NULL, 'LLLL', NULL, 'H6H'),
(3, 1, 'FfqFFFdqUd', NULL, 'h%', NULL, 'vvzqzjSWzHvSvqWHSWj', '_ddL_Ldd__');
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
(1, 1, 'ara', 'Infinity', 1, NULL, 'g', 4095, NULL, 72, NULL, NULL),
(2, 2, 'y', NULL, 1, NULL, '''t2eeYY', 512, 317, 7, 'None', 'NULL'),
(3, 2, 'KLVB', NULL, 1, NULL, NULL, NULL, 128, 2998, 'pv5BvgDvBeVgV', 'Czz2s7SzzwS7zS');
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
(1, 1, 2, NULL, NULL, NULL, 1);
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
(1, 2, 1, 1, 'llODa0alKa');
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
(2, 3, 1, 'Bulgaria', '');
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
(1, 3, 1, '4.0', NULL),
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
(1, 1, 2),
(2, 2, 1),
(3, 3, 1);
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
(1, 1, 3, 3),
(2, 3, 2, 1),
(3, 1, 1, 3);
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
(1, 1, 1, 'YYYYYYYYYYYYYYYY', 'BB'),
(2, 1, 1, '', NULL),
(3, 1, 1, '6''I6mA66I', NULL);
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
(1, '77gd7dg', NULL, NULL, NULL, NULL, NULL),
(2, 'pKKfHhjphpRKpjj', NULL, NULL, 'Scunthorpe', 'Scunthorpe', '9J%d999'),
(3, '__proto__', '[ru]', NULL, NULL, '2', '6rCr2');
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
(1, 'marvel-cinematic-universe', '3'),
(2, 'marvel-cinematic-universe', 'vFTvT');
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
(1, 'Downey Jr., Robert', 'Jw', NULL, 'o', NULL, '''''N2z66O6-6', NULL, NULL);
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
(1, 'Voice Character', NULL, NULL, NULL, '0', NULL),
(2, 'Other Character', 'Mw', 5, '', NULL, '4');
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
(1, 'mUeUwmmeUUUmmeUeUUwwwe', 'f8fiSKG', 1, 2006, 512, NULL, 512, 128, NULL, NULL, 'Inf'),
(2, 'hhhh', NULL, 1, 2007, NULL, NULL, 697, 4369, 511, NULL, 'jj'),
(3, 'JgJJgg''''g', NULL, 1, 2006, NULL, NULL, 2919, 443, 127, NULL, NULL);
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
(1, 1, 'b', NULL, NULL, NULL, NULL, NULL),
(2, 1, '6rCr2', '5', NULL, 'LLLL', NULL, 'H6H'),
(3, 1, 'FfqFFFdqUd', NULL, 'h%', NULL, 'vvzqzjSWzHvSvqWHSWj', '_ddL_Ldd__');
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
(1, 1, 'ara', 'Infinity', 1, NULL, 'g', 4095, NULL, 72, NULL, NULL),
(2, 2, 'y', NULL, 1, NULL, '''t2eeYY', 512, 317, 7, 'None', 'NULL'),
(3, 2, 'KLVB', NULL, 1, NULL, NULL, NULL, 128, 2998, 'pv5BvgDvBeVgV', 'Czz2s7SzzwS7zS');
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
(1, 1, 2, NULL, NULL, NULL, 1);
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
(1, 2, 1, 1, 'llODa0alKa');
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
(2, 3, 1, 'Bulgaria', '');
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
(1, 3, 1, '4.0', NULL),
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
(1, 1, 2),
(2, 2, 1),
(3, 3, 1);
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
(1, 1, 3, 3),
(2, 3, 2, 1),
(3, 1, 1, 3);
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
(1, 1, 1, 'YYYYYYYYYYYYYYYY', 'BB'),
(2, 1, 1, '', NULL),
(3, 1, 1, '6''I6mA66I', NULL);
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
(1, '77gd7dg', NULL, NULL, NULL, NULL, NULL),
(2, 'pKKfHhjphpRKpjj', NULL, NULL, 'Scunthorpe', 'Scunthorpe', '9J%d999'),
(3, '__proto__', '[ru]', NULL, NULL, '2', 'b');
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
(1, 'marvel-cinematic-universe', '3'),
(2, 'marvel-cinematic-universe', 'vFTvT');
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
(1, 'Downey Jr., Robert', 'Jw', NULL, 'o', NULL, '''''N2z66O6-6', NULL, NULL);
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
(1, 'Voice Character', NULL, NULL, NULL, '0', NULL),
(2, 'Other Character', 'Mw', 5, '', NULL, '4');
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
(1, 'mUeUwmmeUUUmmeUeUUwwwe', 'f8fiSKG', 1, 2006, 512, NULL, 512, 128, NULL, NULL, 'Inf'),
(2, 'hhhh', NULL, 1, 2007, NULL, NULL, 697, 4369, 511, NULL, 'jj'),
(3, 'JgJJgg''''g', NULL, 1, 2006, NULL, NULL, 2919, 443, 127, NULL, NULL);
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
(1, 1, 'b', NULL, NULL, NULL, NULL, NULL),
(2, 1, '6rCr2', '5', NULL, 'LLLL', NULL, 'H6H'),
(3, 1, 'FfqFFFdqUd', NULL, 'h%', NULL, 'vvzqzjSWzHvSvqWHSWj', '_ddL_Ldd__');
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
(1, 1, 'ara', 'Infinity', 1, NULL, 'g', 4095, NULL, 72, NULL, NULL),
(2, 2, '''''N2z66O6-6', NULL, 1, NULL, '''t2eeYY', 512, 317, 7, 'None', 'NULL'),
(3, 2, 'KLVB', NULL, 1, NULL, NULL, NULL, 128, 2998, 'pv5BvgDvBeVgV', 'Czz2s7SzzwS7zS');
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
(1, 1, 2, NULL, NULL, NULL, 1);
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
(1, 2, 1, 1, 'llODa0alKa');
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
(2, 3, 1, 'Bulgaria', '');
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
(1, 3, 1, '4.0', NULL),
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
(1, 1, 2),
(2, 2, 1),
(3, 3, 1);
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
(1, 1, 3, 3),
(2, 3, 2, 1),
(3, 1, 1, 3);
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
(1, 1, 1, 'YYYYYYYYYYYYYYYY', 'BB'),
(2, 1, 1, '', NULL),
(3, 1, 1, '6''I6mA66I', NULL);
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
(1, '77gd7dg', NULL, NULL, NULL, NULL, NULL),
(2, 'pKKfHhjphpRKpjj', NULL, NULL, 'Scunthorpe', 'Scunthorpe', '9J%d999'),
(3, '__proto__', '[ru]', NULL, NULL, '2', 'b');
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
(1, 'marvel-cinematic-universe', '3'),
(2, 'marvel-cinematic-universe', 'vFTvT');
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
(1, 'Downey Jr., Robert', 'Jw', NULL, 'o', NULL, '''''N2z66O6-6', NULL, NULL);
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
(1, 'Voice Character', NULL, NULL, NULL, '0', NULL),
(2, 'Other Character', 'Mw', 5, '', NULL, '4');
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
(1, 'mUeUwmmeUUUmmeUeUUwwwe', 'f8fiSKG', 1, 2006, 512, NULL, 512, 128, NULL, NULL, 'Inf'),
(2, 'hhhh', NULL, 1, 2007, NULL, NULL, 697, 4369, 511, NULL, 'jj'),
(3, 'JgJJgg''''g', NULL, 1, 2006, NULL, NULL, 2919, 443, 127, NULL, NULL);
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
(1, 1, 'b', NULL, NULL, NULL, NULL, NULL),
(2, 1, '6rCr2', '5', NULL, 'LLLL', NULL, 'H6H'),
(3, 1, 'FfqFFFdqUd', NULL, 'h%', NULL, 'vvzqzjSWzHvSvqWHSWj', '_ddL_Ldd__');
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
(1, 1, 'ara', 'Infinity', 1, NULL, 'g', 4095, NULL, 72, NULL, NULL),
(2, 2, 'y', NULL, 1, NULL, '''t2eeYY', 512, 317, 1, 'None', 'NULL'),
(3, 2, 'KLVB', NULL, 1, NULL, NULL, NULL, 128, 2998, 'pv5BvgDvBeVgV', 'Czz2s7SzzwS7zS');
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
(1, 1, 2, NULL, NULL, NULL, 1);
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
(1, 2, 1, 1, 'llODa0alKa');
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
(2, 3, 1, 'Bulgaria', '');
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
(1, 3, 1, '4.0', NULL),
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
(1, 1, 2),
(2, 2, 1),
(3, 3, 1);
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
(1, 1, 3, 3),
(2, 3, 2, 1),
(3, 1, 1, 3);
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
(1, 1, 1, 'YYYYYYYYYYYYYYYY', 'BB'),
(2, 1, 1, '', NULL),
(3, 1, 1, '6''I6mA66I', NULL);
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
(1, 'rr', '[ru]', 4287, 'HJ', NULL, NULL);
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
(1, 'marvel-cinematic-universe', ''),
(2, 'character-name-in-title', NULL);
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
(1, 'Other Person', NULL, NULL, 'rt r', 'd5ddd', NULL, 'false', '1GG 11y9GGyGGGy1  G9G1y');
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
(1, 'Voice Character', NULL, NULL, 'Scunthorpe', NULL, NULL);
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
(1, 'null', NULL, 1, NULL, NULL, NULL, 343, 248, NULL, 'NIL', NULL),
(2, 'NUL', NULL, 1, 2012, NULL, '29zyyy2D9zgrDgr9gnyyrDuzy', 180, 1189, 4516, NULL, '__proto__');
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
(1, 1, 'if', '9HH', NULL, '_f86u_m8_uu_', '4k848Xii4XninMXMni', 'Scunthorpe'),
(2, 1, 'KEEEnK', 'aaNffYia11aaNia', '1e100', 'ffW', 'hE99gghI-SgE0ESh', NULL);
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
(1, 1, '', NULL, 1, 37, 'pp11F1', NULL, NULL, 989, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', 48, 1),
(2, 1, 2, 1, NULL, NULL, 1);
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
(1, NULL, 2, 3),
(2, 1, 2, 3),
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
(1, 1, 1, 2, 'DuTVuVuV'),
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 2, 2, 'Bulgaria', '');
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
(1, 1, 2, '4.0', ''),
(2, 1, 3, '4.0', 'gsssssllsd0dgld66ss0'),
(3, 1, 3, '4.0', NULL);
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
(1, 1, 2, 1),
(2, 1, 1, 1),
(3, 2, 1, 1);
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
(1, 1, 2, 'TWTTTTTZTZTT0W0', NULL),
(2, 1, 3, 'H', 'rgjgrHHUrrHrggHggUr');
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
(1, 'rr', '[ru]', 4287, 'HJ', NULL, NULL);
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
(1, 'marvel-cinematic-universe', ''),
(2, 'character-name-in-title', NULL);
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
(1, 'Other Person', NULL, NULL, 'rt r', 'd5ddd', NULL, 'false', '1GG 11y9GGyGGGy1  G9G1y');
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
(1, 'Voice Character', NULL, NULL, 'Scunthorpe', NULL, NULL);
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
(1, 'null', NULL, 1, NULL, NULL, NULL, 343, 248, NULL, 'NIL', NULL),
(2, 'NUL', NULL, 1, 2012, NULL, '29zyyy2D9zgrDgr9gnyyrDuzy', 180, 1189, 4516, NULL, '__proto__');
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
(1, 1, 'if', '9HH', NULL, '_f86u_m8_uu_', '4k848Xii4XninMXMni', 'Scunthorpe'),
(2, 1, 'KEEEnK', 'aaNffYia11aaNia', '1e100', 'ffW', 'hE99gghI-SgE0ESh', NULL);
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
(1, 1, '', NULL, 1, 37, 'pp11F1', NULL, NULL, 989, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', 48, 1),
(2, 1, 2, 1, NULL, NULL, 1);
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
(1, NULL, 2, 3),
(2, 1, 2, 3),
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
(1, 1, 1, 2, 'DuTVuVuV'),
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 2, 2, 'Bulgaria', '');
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
(1, 1, 2, '4.0', ''),
(2, 1, 3, '4.0', 'gsssssllsd0dgld66ss0'),
(3, 1, 3, '4.0', NULL);
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
(1, 1, 2, 1),
(2, 1, 1, 1),
(3, 2, 1, 1);
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
(1, 1, 2, 'TWTTTTTZTZTT0W0', NULL),
(2, 1, 3, 'H', 'rgjgrHHUrrHrggHggUr');
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
(1, 'rr', '[ru]', 4287, 'HJ', NULL, NULL);
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
(1, 'marvel-cinematic-universe', ''),
(2, 'character-name-in-title', NULL);
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
(1, 'Other Person', NULL, NULL, 'rt r', 'd5ddd', NULL, 'false', '1GG 11y9GGyGGGy1  G9G1y');
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
(1, 'Voice Character', NULL, NULL, 'Scunthorpe', NULL, NULL);
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
(1, 'null', NULL, 1, NULL, NULL, NULL, 343, 248, NULL, 'NIL', NULL),
(2, 'NUL', NULL, 1, 2012, NULL, '29zyyy2D9zgrDgr9gnyyrDuzy', 180, 1189, 4516, NULL, '__proto__');
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
(1, 1, 'if', '9HH', NULL, '_f86u_m8_uu_', '4k848Xii4XninMXMni', 'Scunthorpe'),
(2, 1, 'KEEEnK', 'aaNffYia11aaNia', '1e100', 'ffW', 'hE99gghI-SgE0ESh', NULL);
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
(1, 1, '', NULL, 1, 37, 'pp11F1', NULL, NULL, 989, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', 48, 1),
(2, 1, 2, 1, NULL, NULL, 1);
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
(1, NULL, 2, 3),
(2, 1, 2, 3),
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
(1, 1, 1, 2, 'DuTVuVuV'),
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 2, 2, 'Bulgaria', '');
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
(1, 1, 2, '4.0', ''),
(2, 1, 3, '4.0', 'gsssssllsd0dgld66ss0'),
(3, 1, 3, '4.0', NULL);
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
(1, 1, 2, 1),
(2, 1, 1, 1),
(3, 2, 1, 1);
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
(1, 1, 2, 'TWTTTTTZTZTT0W0', NULL),
(2, 1, 3, 'H', '');
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
(1, 'then', NULL, NULL, 'FYVYYVYY  VFFFYFYVFYF', NULL, NULL);
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
(1, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ALAAaM', 9999, '5jZZjS', '51q1B0d''x501dsqy', NULL, '_nDnjz', 'else'),
(2, 'Other Person', 'yYYy', 10000, NULL, 'aBw', NULL, NULL, 's');
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
(1, 'Voice Character', NULL, NULL, 'CC', NULL, 'nil');
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
(1, 'LPT1', NULL, 2, NULL, 2426, 'zzzazz', 957, NULL, NULL, 'INF', NULL),
(2, 'dddd', NULL, 2, NULL, NULL, NULL, NULL, 10000, 2231, 'None', NULL),
(3, '7O', NULL, 1, 2005, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'none', NULL, '666JJC-6JDCJF0CKKCFCK', 'A', NULL, NULL);
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
(1, 2, '333llt', 'qooiqi', 2, NULL, NULL, 468, 2335, NULL, 'iUU', 'XcO'),
(2, 1, 'tKJtKqq', NULL, 2, 0, '', NULL, 9999, 10000, '0', NULL),
(3, 3, 'jWV', '0', 2, 9999, '6y', NULL, 672, 110, NULL, NULL);
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
(1, 2, 2, NULL, '(uncredited)', NULL, 2),
(2, 1, 3, 1, '(voice)', NULL, 2),
(3, 1, 3, NULL, NULL, 19, 1);
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
(1, 2, 3, 3);
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
(1, 3, 1, 1, 'Infinity'),
(2, 2, 1, 1, ''),
(3, 3, 1, 1, '1e100');
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
(1, 1, 1, 'USA', 'FFccF'),
(2, 3, 2, 'USA', NULL);
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
(1, 3, 2, '4.0', 'FALSE');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 1);
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
(1, 1, 2, '6664Nu44NNu', ''),
(2, 2, 2, '000', '8'),
(3, 1, 2, '_BBU_UUB__UB_', NULL);
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
(1, 'then', NULL, NULL, 'FYVYYVYY  VFFFYFYVFYF', NULL, NULL);
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
(1, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ALAAaM', 9999, '5jZZjS', '51q1B0d''x501dsqy', NULL, '_nDnjz', 'else'),
(2, 'Other Person', 'yYYy', 10000, NULL, 'aBw', NULL, NULL, 's');
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
(1, 'Voice Character', NULL, NULL, 'CC', NULL, 'nil');
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
(1, 'LPT1', NULL, 2, NULL, 2426, 'zzzazz', 957, NULL, NULL, 'INF', NULL),
(2, 'dddd', NULL, 2, NULL, NULL, NULL, NULL, 10000, 2231, 'None', NULL),
(3, '7O', NULL, 1, 2005, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'none', NULL, '666JJC-6JDCJF0CKKCFCK', 'A', NULL, NULL);
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
(1, 2, '333llt', 'qooiqi', 2, NULL, '', NULL, 2335, NULL, 'iUU', 'XcO'),
(2, 1, 'tKJtKqq', NULL, 2, 0, '', NULL, 9999, 10000, '0', NULL),
(3, 3, 'jWV', '0', 2, 9999, '6y', NULL, 672, 110, NULL, NULL);
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
(1, 2, 2, NULL, '(uncredited)', NULL, 2),
(2, 1, 3, 1, '(voice)', NULL, 2),
(3, 1, 3, NULL, NULL, 19, 1);
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
(1, 2, 3, 3);
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
(1, 3, 1, 1, 'Infinity'),
(2, 2, 1, 1, ''),
(3, 3, 1, 1, '1e100');
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
(1, 1, 1, 'USA', 'FFccF'),
(2, 3, 2, 'USA', NULL);
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
(1, 3, 2, '4.0', 'FALSE');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 1);
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
(1, 1, 2, '6664Nu44NNu', ''),
(2, 2, 2, '000', '8'),
(3, 1, 2, '_BBU_UUB__UB_', NULL);
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
(1, 'then', NULL, NULL, 'FYVYYVYY  VFFFYFYVFYF', NULL, NULL);
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
(1, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ALAAaM', 9999, '5jZZjS', '51q1B0d''x501dsqy', NULL, '_nDnjz', 'else'),
(2, 'Other Person', 'yYYy', 10000, NULL, 'aBw', NULL, NULL, 's');
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
(1, 'Voice Character', NULL, NULL, 'CC', NULL, 'nil');
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
(1, 'LPT1', NULL, 2, NULL, 2426, 'zzzazz', 957, NULL, NULL, 'INF', NULL),
(2, 'dddd', NULL, 2, NULL, NULL, NULL, NULL, 10000, 2231, 'None', NULL),
(3, '7O', NULL, 1, 2005, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'none', NULL, '666JJC-6JDCJF0CKKCFCK', 'A', NULL, NULL);
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
(1, 2, '333llt', 'qooiqi', 2, NULL, NULL, 468, 2335, NULL, 'iUU', 'XcO'),
(2, 1, 'tKJtKqq', NULL, 2, 0, '', NULL, 9999, 10000, '0', NULL),
(3, 3, 'jWV', '0', 2, 9999, '6y', NULL, 672, 110, NULL, NULL);
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
(1, 2, 2, NULL, '(uncredited)', NULL, 2),
(2, 1, 3, 1, '(voice)', NULL, 2),
(3, 1, 3, NULL, NULL, 19, 1);
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
(1, 2, 3, 3);
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
(1, 3, 1, 1, 'Infinity'),
(2, 2, 1, 1, ''),
(3, 3, 1, 1, '1e100');
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
(1, 1, 1, 'USA', 'FFccF');
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
(1, 3, 2, '4.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 1);
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
(1, 2, 2, '1', NULL),
(2, 1, 1, '1', NULL),
(3, 2, 2, '1', NULL);
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
(1, 'then', NULL, NULL, 'FYVYYVYY  VFFFYFYVFYF', NULL, NULL);
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
(1, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ALAAaM', 9999, '5jZZjS', '51q1B0d''x501dsqy', NULL, '_nDnjz', 'else'),
(2, 'Other Person', 'yYYy', 10000, NULL, 'aBw', NULL, NULL, 's');
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
(1, 'Voice Character', NULL, NULL, 'CC', NULL, 'nil');
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
(1, 'LPT1', NULL, 2, NULL, 2426, 'zzzazz', 957, NULL, NULL, 'INF', NULL),
(2, 'dddd', NULL, 2, NULL, NULL, NULL, NULL, 10000, 2231, 'None', NULL),
(3, '7O', NULL, 1, 2005, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'none', NULL, '666JJC-6JDCJF0CKKCFCK', 'A', NULL, NULL);
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
(1, 2, '333llt', 'qooiqi', 2, NULL, NULL, 468, 2335, NULL, 'iUU', 'XcO'),
(2, 1, 'tKJtKqq', NULL, 2, 0, '', NULL, 9999, 10000, '0', NULL),
(3, 3, 'jWV', '0', 2, 9999, '6y', NULL, 672, 110, NULL, NULL);
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
(1, 2, 2, NULL, '(uncredited)', NULL, 2),
(2, 1, 3, 1, '(voice)', NULL, 2),
(3, 1, 3, NULL, NULL, 19, 1);
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
(1, 2, 3, 3);
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
(1, 3, 1, 1, 'Infinity'),
(2, 2, 1, 1, ''),
(3, 3, 1, 1, '1e100');
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
(1, 1, 1, 'USA', 'FFccF'),
(2, 3, 2, 'USA', NULL);
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
(1, 3, 2, '4.0', 'FALSE');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 3, 1);
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
(1, 1, 2, '6664Nu44NNu', ''),
(2, 2, 2, '000', '8'),
(3, 1, 2, '7O', NULL);
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
(1, 'DttDDtDtDDtD', '[de]', 1, NULL, NULL, NULL),
(2, 'ZZZZZZZZZZZZZZ', '[us]', NULL, '', NULL, 'NULL'),
(3, 'null', '[de]', 128, NULL, '__dict__', NULL);
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
(1, 'marvel-cinematic-universe', 'yFnnjjqUyyyUnq');
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
(1, 'Other Person', 'aaassasaaasaas', 5, NULL, 'COM1', 'Qnp00nnpn', 'l', 'x');
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
(1, 'Other Character', 'RiRT4HfHHnH_', 10000, NULL, 'wwwwwww', NULL);
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
(1, 'NUL', NULL, 2, 2012, 0, NULL, NULL, 4095, 127, NULL, 'h'),
(2, '0', NULL, 1, 2011, 4095, NULL, NULL, NULL, NULL, '__dict__', 'FX1gdg');
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
(1, 1, 'm2z', NULL, NULL, '', '0', NULL),
(2, 1, 'S_q', 'True', 'INF', 'KUKC0K', NULL, NULL);
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
(1, 2, 'Scunthorpe', NULL, 2, NULL, NULL, NULL, 512, 8191, NULL, NULL),
(2, 2, 'Scunthorpe', '1e100', 2, 6, NULL, NULL, 2, 8192, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(2, 1, 1, 3),
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
(1, 1, 1, 1, 'xjjojxx');
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 1, 1, 'Bulgaria', 'WWWWWWWW');
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
(1, 2, 1);
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
(1, 1, 2, 2),
(2, 2, 1, 2);
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
(1, 1, 1, 'NNOB', NULL),
(2, 1, 1, 'None', 'None');
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
(1, 'DttDDtDtDDtD', '[de]', 1, NULL, NULL, NULL),
(2, 'ZZZZZZZZZZZZZZ', '[us]', NULL, '', NULL, 'NULL'),
(3, 'null', '[de]', 128, NULL, '__dict__', NULL);
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
(1, 'marvel-cinematic-universe', 'yFnnjjqUyyyUnq');
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
(1, 'Other Person', 'aaassasaaasaas', 5, NULL, 'COM1', 'Qnp00nnpn', 'l', 'x');
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
(1, 'Other Character', 'RiRT4HfHHnH_', 10000, NULL, 'wwwwwww', NULL);
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
(1, 'NUL', NULL, 2, 2012, 0, NULL, NULL, 4095, 127, NULL, 'h'),
(2, '0', NULL, 1, 2011, 4095, NULL, NULL, NULL, NULL, '__dict__', 'FX1gdg');
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
(1, 1, 'm2z', NULL, NULL, '', '0', NULL),
(2, 1, 'S_q', 'True', 'INF', 'KUKC0K', NULL, NULL);
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
(1, 2, 'Scunthorpe', NULL, 2, NULL, NULL, NULL, 512, 8191, NULL, NULL),
(2, 2, 'Scunthorpe', '1e100', 2, 6, NULL, NULL, 2, 8192, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(2, 1, 1, 2),
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
(1, 1, 1, 1, 'xjjojxx');
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 1, 1, 'Bulgaria', 'WWWWWWWW');
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
(1, 2, 1);
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
(1, 1, 2, 2),
(2, 2, 1, 2);
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
(1, 1, 1, 'NNOB', NULL),
(2, 1, 1, 'None', 'None');
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
(1, 'DttDDtDtDDtD', '[de]', 1, NULL, NULL, NULL),
(2, 'ZZZZZZZZZZZZZZ', '[us]', NULL, '', NULL, 'NULL'),
(3, 'null', '[de]', 128, NULL, '__dict__', NULL);
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
(1, 'marvel-cinematic-universe', 'yFnnjjqUyyyUnq');
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
(1, 'Other Person', 'aaassasaaasaas', 5, NULL, 'COM1', 'Qnp00nnpn', 'l', 'x');
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
(1, 'Other Character', 'RiRT4HfHHnH_', 10000, NULL, 'wwwwwww', NULL);
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
(1, 'NUL', NULL, 2, 2012, 0, NULL, NULL, 4095, 127, NULL, 'h'),
(2, '0', NULL, 1, 2011, 4095, NULL, NULL, NULL, NULL, '__dict__', 'FX1gdg');
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
(1, 1, 'm2z', NULL, NULL, '', '0', NULL),
(2, 1, 'S_q', 'True', 'h', 'KUKC0K', NULL, NULL);
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
(1, 2, 'Scunthorpe', NULL, 2, NULL, NULL, NULL, 512, 8191, NULL, NULL),
(2, 2, 'Scunthorpe', '1e100', 2, 6, NULL, NULL, 2, 8192, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(2, 1, 1, 3),
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
(1, 1, 1, 1, 'xjjojxx');
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 1, 1, 'Bulgaria', 'WWWWWWWW');
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
(1, 2, 1);
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
(1, 1, 2, 2),
(2, 2, 1, 2);
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
(1, 1, 1, 'NNOB', NULL),
(2, 1, 1, 'None', 'None');
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
(1, 'complete'),
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
(1, 'DttDDtDtDDtD', '[de]', 1, NULL, NULL, NULL),
(2, 'ZZZZZZZZZZZZZZ', '[us]', NULL, '', NULL, 'NULL'),
(3, 'null', '[de]', 128, NULL, '__dict__', NULL);
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
(1, 'marvel-cinematic-universe', 'yFnnjjqUyyyUnq');
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
(1, 'Other Person', 'aaassasaaasaas', 5, NULL, 'COM1', 'Qnp00nnpn', 'l', 'x');
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
(1, 'Other Character', 'RiRT4HfHHnH_', 10000, NULL, 'wwwwwww', NULL);
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
(1, 'NUL', NULL, 2, 2012, 0, NULL, NULL, 4095, 127, NULL, 'h'),
(2, '0', NULL, 1, 2011, 4095, NULL, NULL, NULL, NULL, '__dict__', 'FX1gdg');
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
(1, 1, 'm2z', NULL, NULL, '', '', NULL),
(2, 1, '1', NULL, 'True', 'INF', 'KUKC0K', NULL);
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
(1, 2, '1', NULL, 2, NULL, NULL, NULL, NULL, 512, '1', NULL),
(2, 2, 'Scunthorpe', '1e100', 2, 6, NULL, NULL, 2, 8192, NULL, NULL);
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
(1, 1, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(2, 1, 1, 3),
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
(1, 1, 1, 1, 'xjjojxx');
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
(1, 2, 1, 'Bulgaria', NULL),
(2, 1, 1, 'Bulgaria', 'WWWWWWWW');
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
(1, 2, 1);
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
(1, 1, 2, 2),
(2, 2, 1, 2);
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
(1, 1, 1, 'NNOB', NULL),
(2, 1, 1, 'None', 'None');
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
(1, '__dict__', '[ru]', 149, NULL, '', 'True'),
(2, '-Infinity', NULL, 633, NULL, 'Scunthorpe', NULL),
(3, '22DD2C22DDDDDC22DCC', '[us]', 860, NULL, NULL, NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'lflxqxBlxxB'),
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
(1, 'Downey Jr., Robert', 'TTTTT', 143, 'UnExxeUrUr', 'A_I', NULL, '__proto__', 'F67F 7'),
(2, 'Other Person', NULL, NULL, NULL, '00', NULL, 'PQQ Q', NULL),
(3, 'Downey Jr., Robert', NULL, 16, 'VvvzVzvzUv', 'oSNSooSSS', NULL, NULL, NULL);
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
(1, 'Other Character', NULL, 473, NULL, '2', NULL);
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
(1, 'B', NULL, 1, NULL, 5, '_TfTS_ST_TSSf_fSTS', NULL, 3951, NULL, 'SdSa6adjaa6SjS', '__dict__'),
(2, 'true', '', 1, NULL, NULL, NULL, NULL, 8191, NULL, NULL, NULL);
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
(1, 3, '1e100', NULL, NULL, '%-aea8', NULL, 'ssKO'),
(2, 2, 'IIggo', NULL, '', NULL, 'eTTTTyeTETeTEyETTTTeyEyTy', NULL);
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
(1, 2, 'Z', '', 2, 16, 'Inf', 6, 1716, 64, NULL, NULL),
(2, 1, '6Nj% G% GN6jGPN', 'f', 1, NULL, 'u-', NULL, NULL, 0, NULL, NULL);
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
(1, 2, 2, 1, '(voice) (uncredited)', NULL, 1),
(2, 2, 1, NULL, '(uncredited)', NULL, 1);
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
(2, 2, 2, 1),
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
(1, 2, 1, 1, NULL);
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
(1, 1, 1, 'Bulgaria', 'True'),
(2, 2, 2, 'USA', 'r');
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
(1, 2, 2, '4.0', 'TUnnpOndnpUpnn'),
(2, 2, 2, '8.0', NULL);
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
(1, 3, 3, 'TM8jbbM8TjMjj888TMjTj8jTj8Tb', 'f'),
(2, 1, 2, 'if', '2'),
(3, 3, 3, 'COM1', NULL);
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
(1, '__dict__', '[ru]', 149, NULL, '', 'True'),
(2, '-Infinity', NULL, 633, NULL, 'Scunthorpe', NULL),
(3, '22DD2C22DDDDDC22DCC', '[us]', 860, NULL, NULL, NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'lflxqxBlxxB'),
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
(1, 'Downey Jr., Robert', 'TTTTT', 143, 'UnExxeUrUr', 'A_I', NULL, '__proto__', 'F67F 7'),
(2, 'Other Person', NULL, NULL, NULL, '00', NULL, 'PQQ Q', NULL),
(3, 'Downey Jr., Robert', NULL, 16, 'VvvzVzvzUv', 'oSNSooSSS', NULL, NULL, NULL);
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
(1, 'Other Character', NULL, 473, NULL, '2', NULL);
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
(1, 'B', NULL, 1, NULL, 5, '_TfTS_ST_TSSf_fSTS', NULL, 3951, NULL, '6Nj% G% GN6jGPN', '__dict__'),
(2, 'true', '', 1, NULL, NULL, NULL, NULL, 8191, NULL, NULL, NULL);
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
(1, 3, '1e100', NULL, NULL, '%-aea8', NULL, 'ssKO'),
(2, 2, 'IIggo', NULL, '', NULL, 'eTTTTyeTETeTEyETTTTeyEyTy', NULL);
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
(1, 2, 'Z', '', 2, 16, 'Inf', 6, 1716, 64, NULL, NULL),
(2, 1, '6Nj% G% GN6jGPN', 'f', 1, NULL, 'u-', NULL, NULL, 0, NULL, NULL);
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
(1, 2, 2, 1, '(voice) (uncredited)', NULL, 1),
(2, 2, 1, NULL, '(uncredited)', NULL, 1);
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
(2, 2, 2, 1),
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
(1, 2, 1, 1, NULL);
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
(1, 1, 1, 'Bulgaria', 'True'),
(2, 2, 2, 'USA', 'r');
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
(1, 2, 2, '4.0', 'TUnnpOndnpUpnn'),
(2, 2, 2, '8.0', NULL);
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
(1, 3, 3, 'TM8jbbM8TjMjj888TMjTj8jTj8Tb', 'f'),
(2, 1, 2, 'if', '2'),
(3, 3, 3, 'COM1', NULL);
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
(1, '__dict__', '[ru]', 149, NULL, '', 'True'),
(2, '-Infinity', NULL, 633, NULL, 'Scunthorpe', NULL),
(3, '22DD2C22DDDDDC22DCC', '[us]', 860, NULL, NULL, NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'lflxqxBlxxB'),
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
(1, 'Downey Jr., Robert', 'TTTTT', 143, 'UnExxeUrUr', 'A_I', NULL, '__proto__', 'F67F 7'),
(2, 'Other Person', NULL, NULL, NULL, '00', NULL, 'PQQ Q', NULL),
(3, 'Downey Jr., Robert', NULL, 16, 'VvvzVzvzUv', 'oSNSooSSS', NULL, NULL, NULL);
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
(1, 'Other Character', NULL, 473, NULL, '2', NULL);
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
(1, 'B', NULL, 1, NULL, 5, '_TfTS_ST_TSSf_fSTS', NULL, 3951, NULL, 'SdSa6adjaa6SjS', '__dict__'),
(2, 'true', '', 1, NULL, NULL, NULL, NULL, 8191, NULL, NULL, NULL);
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
(1, 3, '1e100', NULL, NULL, '%-aea8', NULL, 'ssKO'),
(2, 2, 'IIggo', NULL, '', NULL, 'eTTTTyeTETeTEyETTTTeyEyTy', NULL);
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
(1, 2, 'Z', '', 2, 16, 'r', 6, 1716, 64, NULL, NULL),
(2, 1, '6Nj% G% GN6jGPN', 'f', 1, NULL, 'u-', NULL, NULL, 0, NULL, NULL);
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
(1, 2, 2, 1, '(voice) (uncredited)', NULL, 1),
(2, 2, 1, NULL, '(uncredited)', NULL, 1);
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
(2, 2, 2, 1),
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
(1, 2, 1, 1, NULL);
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
(1, 1, 1, 'Bulgaria', 'True'),
(2, 2, 2, 'USA', 'r');
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
(1, 2, 2, '4.0', 'TUnnpOndnpUpnn'),
(2, 2, 2, '8.0', NULL);
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
(1, 3, 3, 'TM8jbbM8TjMjj888TMjTj8jTj8Tb', 'f'),
(2, 1, 2, 'if', '2'),
(3, 3, 3, 'COM1', NULL);
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
(1, '__dict__', '[ru]', 149, NULL, '', 'True'),
(2, '-Infinity', NULL, 633, NULL, 'Scunthorpe', NULL),
(3, '22DD2C22DDDDDC22DCC', '[us]', 860, NULL, NULL, NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'lflxqxBlxxB'),
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
(1, 'Downey Jr., Robert', 'TTTTT', 143, 'UnExxeUrUr', 'A_I', NULL, '__proto__', 'F67F 7'),
(2, 'Other Person', NULL, NULL, NULL, '00', NULL, 'PQQ Q', NULL),
(3, 'Downey Jr., Robert', NULL, 16, 'VvvzVzvzUv', 'oSNSooSSS', NULL, NULL, NULL);
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
(1, 'Other Character', NULL, 473, NULL, '2', NULL);
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
(1, 'B', NULL, 1, NULL, 5, '_TfTS_ST_TSSf_fSTS', NULL, 3951, NULL, 'SdSa6adjaa6SjS', '__dict__'),
(2, 'true', NULL, 2, NULL, NULL, NULL, NULL, NULL, 8191, NULL, NULL);
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
(1, 2, '1', NULL, NULL, NULL, '%-aea8', NULL),
(2, 2, 'ssKO', NULL, NULL, NULL, NULL, '');
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
(1, 2, '', NULL, 2, NULL, NULL, NULL, NULL, 1, NULL, '1'),
(2, 2, '1', NULL, 2, NULL, '1', NULL, NULL, NULL, NULL, 'f');
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
(1, 1, 2, 1, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, NULL, 2);
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
(1, 2, 2, 1, NULL);
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
(1, 1, 2, '4.0', NULL),
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
(1, 2, 2),
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
(1, 2, 2, '1', NULL),
(2, 2, 2, '1', NULL),
(3, 2, 2, '1', NULL);
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
(1, 'else', '[de]', NULL, '-58Z5', 'iiiiiiiiii', 'KCrRRURKRCURKr'),
(2, 'INF', '[us]', 3, 'eIeI', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1e100');
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
(1, 'Downey Jr., Robert', 'x7xg77xgxxEgEEEg7E', 63, NULL, '', NULL, NULL, 'Xv');
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
(1, 'Voice Character', NULL, 1303, '   ', NULL, 'a88RRa'),
(2, 'Other Character', 'a7mhhhn7nnmhhmmn', NULL, NULL, 'a_A55__''O5t''', NULL);
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
(1, '4x4R', NULL, 3, 2005, NULL, NULL, 5937, NULL, 2637, NULL, NULL),
(2, '8n-n-8rr887-O-78', 'null', 3, NULL, 15, NULL, 686, 153, NULL, NULL, NULL),
(3, ' ', 'ZZZZZ', 1, 2005, NULL, NULL, NULL, NULL, 847, NULL, NULL);
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
(1, 1, 'if', NULL, '', '', NULL, NULL),
(2, 1, 'u', 'LPT1', NULL, 'lE3E', '0', 'AZAARZ'),
(3, 1, 'NIL', 'UU----UUUU', 'NIL', 'KAKuuKu', '%%', NULL);
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
(1, 2, 'xbbxQxbN8QQuNuuu', '', 2, NULL, 'IB0Q', NULL, 396, NULL, NULL, 'if'),
(2, 3, 'KU_OK0U_U_yKU9yKyKO00Uy9O9_', '999999999999999999999999999999', 1, NULL, 'qAqAlVV2Al2AqA', NULL, 1345, NULL, 'NaN', 'DDDDDD');
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
(1, 1, 3, NULL, '(voice)', 222, 1),
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
(1, 2, 2, 2, '0'),
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
(1, 2, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '8.0', 'none'),
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
(1, 1, 1),
(2, 3, 1),
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
(1, 3, 1, 3);
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
(1, 1, 1, 'Wm', NULL);
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
(1, 'else', '[de]', NULL, '-58Z5', 'iiiiiiiiii', 'UU----UUUU'),
(2, 'INF', '[us]', 3, 'eIeI', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1e100');
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
(1, 'Downey Jr., Robert', 'x7xg77xgxxEgEEEg7E', 63, NULL, '', NULL, NULL, 'Xv');
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
(1, 'Voice Character', NULL, 1303, '   ', NULL, 'a88RRa'),
(2, 'Other Character', 'a7mhhhn7nnmhhmmn', NULL, NULL, 'a_A55__''O5t''', NULL);
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
(1, '4x4R', NULL, 3, 2005, NULL, NULL, 5937, NULL, 2637, NULL, NULL),
(2, '8n-n-8rr887-O-78', 'null', 3, NULL, 15, NULL, 686, 153, NULL, NULL, NULL),
(3, ' ', 'ZZZZZ', 1, 2005, NULL, NULL, NULL, NULL, 847, NULL, NULL);
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
(1, 1, 'if', NULL, '', '', NULL, NULL),
(2, 1, 'u', 'LPT1', NULL, 'lE3E', '0', 'AZAARZ'),
(3, 1, 'NIL', 'UU----UUUU', 'NIL', 'KAKuuKu', '%%', NULL);
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
(1, 2, 'xbbxQxbN8QQuNuuu', '', 2, NULL, 'IB0Q', NULL, 396, NULL, NULL, 'if'),
(2, 3, 'KU_OK0U_U_yKU9yKyKO00Uy9O9_', '999999999999999999999999999999', 1, NULL, 'qAqAlVV2Al2AqA', NULL, 1345, NULL, 'NaN', 'DDDDDD');
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
(1, 1, 3, NULL, '(voice)', 222, 1),
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
(1, 2, 2, 2, '0'),
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
(1, 2, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '8.0', 'none'),
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
(1, 1, 1),
(2, 3, 1),
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
(1, 3, 1, 3);
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
(1, 1, 1, 'Wm', NULL);
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
(1, 'else', '[de]', NULL, '-58Z5', 'iiiiiiiiii', 'KCrRRURKRCURKr'),
(2, 'INF', '[us]', 3, 'eIeI', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1e100');
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
(1, 'Downey Jr., Robert', 'x7xg77xgxxEgEEEg7E', 63, NULL, '', NULL, NULL, 'Xv');
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
(1, 'Voice Character', NULL, 1303, '   ', NULL, 'a88RRa'),
(2, 'Other Character', 'a7mhhhn7nnmhhmmn', NULL, NULL, 'a_A55__''O5t''', NULL);
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
(1, '4x4R', NULL, 3, 2005, NULL, NULL, 5937, NULL, 2637, NULL, NULL),
(2, '8n-n-8rr887-O-78', 'null', 3, NULL, 15, NULL, 686, 153, NULL, NULL, NULL),
(3, ' ', 'ZZZZZ', 1, 2005, NULL, NULL, NULL, NULL, 847, NULL, NULL);
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
(1, 1, 'if', NULL, '', '', NULL, NULL),
(2, 1, 'u', 'LPT1', NULL, 'lE3E', '0', 'AZAARZ'),
(3, 1, 'NIL', 'UU----UUUU', 'KCrRRURKRCURKr', 'KAKuuKu', '%%', NULL);
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
(1, 2, 'xbbxQxbN8QQuNuuu', '', 2, NULL, 'IB0Q', NULL, 396, NULL, NULL, 'if'),
(2, 3, 'KU_OK0U_U_yKU9yKyKO00Uy9O9_', '999999999999999999999999999999', 1, NULL, 'qAqAlVV2Al2AqA', NULL, 1345, NULL, 'NaN', 'DDDDDD');
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
(1, 1, 3, NULL, '(voice)', 222, 1),
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
(1, 2, 2, 2, '0'),
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
(1, 2, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '8.0', 'none'),
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
(1, 1, 1),
(2, 3, 1),
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
(1, 3, 1, 3);
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
(1, 1, 1, 'Wm', NULL);
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
(1, 'else', '[de]', NULL, '-58Z5', 'iiiiiiiiii', 'KCrRRURKRCURKr'),
(2, 'INF', '[us]', 3, 'eIeI', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1e100');
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
(1, 'Downey Jr., Robert', 'x7xg77xgxxEgEEEg7E', 63, NULL, '', NULL, NULL, 'Xv');
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
(1, 'Voice Character', NULL, 1303, '   ', NULL, 'a88RRa'),
(2, 'Other Character', 'LPT1', NULL, NULL, 'a_A55__''O5t''', NULL);
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
(1, '4x4R', NULL, 3, 2005, NULL, NULL, 5937, NULL, 2637, NULL, NULL),
(2, '8n-n-8rr887-O-78', 'null', 3, NULL, 15, NULL, 686, 153, NULL, NULL, NULL),
(3, ' ', 'ZZZZZ', 1, 2005, NULL, NULL, NULL, NULL, 847, NULL, NULL);
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
(1, 1, 'if', NULL, '', '', NULL, NULL),
(2, 1, 'u', 'LPT1', NULL, 'lE3E', '0', 'AZAARZ'),
(3, 1, 'NIL', 'UU----UUUU', 'NIL', 'KAKuuKu', '%%', NULL);
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
(1, 2, 'xbbxQxbN8QQuNuuu', '', 2, NULL, 'IB0Q', NULL, 396, NULL, NULL, 'if'),
(2, 3, 'KU_OK0U_U_yKU9yKyKO00Uy9O9_', '999999999999999999999999999999', 1, NULL, 'qAqAlVV2Al2AqA', NULL, 1345, NULL, 'NaN', 'DDDDDD');
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
(1, 1, 3, NULL, '(voice)', 222, 1),
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
(1, 2, 2, 2, '0'),
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
(1, 2, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '8.0', 'none'),
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
(1, 1, 1),
(2, 3, 1),
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
(1, 3, 1, 3);
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
(1, 1, 1, 'Wm', NULL);
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
(1, 'else', '[de]', NULL, '-58Z5', 'iiiiiiiiii', 'KCrRRURKRCURKr'),
(2, 'qAqAlVV2Al2AqA', '[us]', 3, 'eIeI', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1e100');
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
(1, 'Downey Jr., Robert', 'x7xg77xgxxEgEEEg7E', 63, NULL, '', NULL, NULL, 'Xv');
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
(1, 'Voice Character', NULL, 1303, '   ', NULL, 'a88RRa'),
(2, 'Other Character', 'a7mhhhn7nnmhhmmn', NULL, NULL, 'a_A55__''O5t''', NULL);
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
(1, '4x4R', NULL, 3, 2005, NULL, NULL, 5937, NULL, 2637, NULL, NULL),
(2, '8n-n-8rr887-O-78', 'null', 3, NULL, 15, NULL, 686, 153, NULL, NULL, NULL),
(3, ' ', 'ZZZZZ', 1, 2005, NULL, NULL, NULL, NULL, 847, NULL, NULL);
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
(1, 1, 'if', NULL, '', '', NULL, NULL),
(2, 1, 'u', 'LPT1', NULL, 'lE3E', '0', 'AZAARZ'),
(3, 1, 'NIL', 'UU----UUUU', 'NIL', 'KAKuuKu', '%%', NULL);
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
(1, 2, 'xbbxQxbN8QQuNuuu', '', 2, NULL, 'IB0Q', NULL, 396, NULL, NULL, 'if'),
(2, 3, 'KU_OK0U_U_yKU9yKyKO00Uy9O9_', '999999999999999999999999999999', 1, NULL, 'qAqAlVV2Al2AqA', NULL, 1345, NULL, 'NaN', 'DDDDDD');
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
(1, 1, 3, NULL, '(voice)', 222, 1),
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
(1, 2, 2, 2, '0'),
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
(1, 2, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '8.0', 'none'),
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
(1, 1, 1),
(2, 3, 1),
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
(1, 3, 1, 3);
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
(1, 1, 1, 'Wm', NULL);
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
(1, 'else', '[de]', NULL, '-58Z5', 'iiiiiiiiii', 'KCrRRURKRCURKr'),
(2, 'INF', '[us]', 3, 'eIeI', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1e100');
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
(1, 'Downey Jr., Robert', 'x7xg77xgxxEgEEEg7E', 63, NULL, '', NULL, NULL, 'Xv');
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
(1, 'Voice Character', NULL, 1303, '   ', NULL, 'a88RRa'),
(2, 'Other Character', 'a7mhhhn7nnmhhmmn', NULL, NULL, 'a_A55__''O5t''', NULL);
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
(1, '4x4R', NULL, 3, 2005, NULL, NULL, 5937, NULL, 2637, NULL, NULL),
(2, '8n-n-8rr887-O-78', 'null', 3, NULL, 15, NULL, 686, 153, NULL, NULL, NULL),
(3, ' ', 'ZZZZZ', 1, 2005, NULL, NULL, NULL, NULL, 847, NULL, NULL);
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
(1, 1, 'if', NULL, '', '', NULL, NULL),
(2, 1, 'u', 'LPT1', NULL, 'lE3E', '0', 'AZAARZ'),
(3, 1, 'NIL', 'UU----UUUU', 'NIL', 'a88RRa', '%%', NULL);
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
(1, 2, 'xbbxQxbN8QQuNuuu', '', 2, NULL, 'IB0Q', NULL, 396, NULL, NULL, 'if'),
(2, 3, 'KU_OK0U_U_yKU9yKyKO00Uy9O9_', '999999999999999999999999999999', 1, NULL, 'qAqAlVV2Al2AqA', NULL, 1345, NULL, 'NaN', 'DDDDDD');
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
(1, 1, 3, NULL, '(voice)', 222, 1),
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
(1, 2, 2, 2, '0'),
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
(1, 2, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '8.0', 'none'),
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
(1, 1, 1),
(2, 3, 1),
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
(1, 3, 1, 3);
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
(1, 1, 1, 'Wm', NULL);
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
(1, '999999999999999999999999999999', '[us]', NULL, NULL, NULL, NULL),
(2, '', '[de]', NULL, 'W7kwPP3WS7k3wkwk', NULL, 'BBBBB');
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', '1e100'),
(3, 'hero-sequel', 'T');
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
(1, 'Other Person', 'B', NULL, NULL, NULL, NULL, NULL, 'i'),
(2, 'Other Person', NULL, 347, '_i_Ii', NULL, '6', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'GGGG', 'undefined', 'rEs5orsHxs5Trx');
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
(1, 'Voice Character', NULL, NULL, 'UKxUK', NULL, NULL);
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
(1, 'TRUE', 'K KGk ', 2, 2005, NULL, 'D_DZeeDe_LDZZZDD_De_eDLLD', 393, 6, 236, NULL, NULL),
(2, 'rrr', 'Infinity', 2, NULL, 2609, NULL, NULL, 4504, 127, 'kkB', NULL),
(3, 'KCC', NULL, 1, NULL, 4933, NULL, NULL, 6, 598, 'TPsv%vNv%Pvs', 'v v');
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
(1, 1, 'EE', 'kkkkkk', '-Infinity', 'Wx34x4gW3', NULL, 'ls2'),
(2, 1, 'oFRFF', NULL, NULL, NULL, NULL, 'TRUE');
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
(1, 1, 'if', NULL, 2, NULL, NULL, 2193, NULL, NULL, 'if', 'zHW'),
(2, 3, 'Y', NULL, 1, 93, 'fd', 8238, 6029, 5417, 'TYTOHYYTSn', NULL),
(3, 2, 'FALSE', NULL, 1, 1345, NULL, 88, 386, 210, '9', NULL);
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
(1, 3, 3, NULL, '(voice) (uncredited)', 9288, 2);
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
(1, 3, 1, 1, 'y9v'),
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
(1, 1, 1, 'USA', NULL),
(2, 3, 1, 'USA', NULL);
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
(2, 3, 1, '4.0', 'C7RAAC7ARR'),
(3, 2, 2, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 2);
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
(1, 2, 3, 1),
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
(1, 3, 1, 'j--_j_--j33jWj', NULL);
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
(1, '999999999999999999999999999999', '[us]', NULL, NULL, NULL, NULL),
(2, '', '[de]', NULL, 'W7kwPP3WS7k3wkwk', NULL, 'BBBBB');
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', '1e100'),
(3, 'hero-sequel', 'T');
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
(1, 'Other Person', 'B', NULL, NULL, NULL, NULL, NULL, 'i'),
(2, 'Other Person', NULL, 347, '_i_Ii', NULL, '6', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'GGGG', 'undefined', 'rEs5orsHxs5Trx');
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
(1, 'Voice Character', NULL, NULL, 'UKxUK', NULL, NULL);
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
(1, 'TRUE', 'K KGk ', 2, 2005, NULL, 'D_DZeeDe_LDZZZDD_De_eDLLD', 393, 6, 236, NULL, NULL),
(2, 'rrr', 'Infinity', 2, NULL, 2609, NULL, NULL, 4504, 127, 'kkB', NULL),
(3, 'KCC', NULL, 1, NULL, 4933, NULL, NULL, 6, 598, 'TPsv%vNv%Pvs', '');
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
(1, 2, '1', NULL, 'kkkkkk', '-Infinity', 'Wx34x4gW3', NULL),
(2, 2, 'ls2', NULL, NULL, NULL, NULL, NULL);
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
(1, 2, '1', NULL, 2, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 2, '1', NULL, 2, NULL, NULL, 1, NULL, NULL, NULL, NULL),
(3, 2, '1', NULL, 2, NULL, NULL, 8238, 6029, 5417, 'TYTOHYYTSn', NULL);
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
(1, 2, 2, 2, NULL),
(2, 2, 2, 3, NULL);
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
(1, 2, 2, 'Bulgaria', '1'),
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
(2, 1, 1, '4.0', NULL),
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
(1, 2, 1);
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
(2, 3, 1, 1);
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
-- Generated database 050/100
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
(1, '999999999999999999999999999999', '[us]', NULL, NULL, NULL, NULL),
(2, '', '[de]', NULL, 'W7kwPP3WS7k3wkwk', NULL, 'BBBBB');
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', '1e100'),
(3, 'hero-sequel', 'T');
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
(1, 'Other Person', 'B', NULL, NULL, NULL, NULL, NULL, 'i'),
(2, 'Other Person', NULL, 347, '_i_Ii', NULL, 'BBBBB', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'GGGG', 'undefined', 'rEs5orsHxs5Trx');
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
(1, 'Voice Character', NULL, NULL, 'UKxUK', NULL, NULL);
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
(1, 'TRUE', 'K KGk ', 2, 2005, NULL, 'D_DZeeDe_LDZZZDD_De_eDLLD', 393, 6, 236, NULL, NULL),
(2, 'rrr', 'Infinity', 2, NULL, 2609, NULL, NULL, 4504, 127, 'kkB', NULL),
(3, 'KCC', NULL, 1, NULL, 4933, NULL, NULL, 6, 598, 'TPsv%vNv%Pvs', 'v v');
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
(1, 1, 'EE', 'kkkkkk', '-Infinity', 'Wx34x4gW3', NULL, 'ls2'),
(2, 1, 'oFRFF', NULL, NULL, NULL, NULL, 'TRUE');
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
(1, 1, 'if', NULL, 2, NULL, NULL, 2193, NULL, NULL, 'if', 'zHW'),
(2, 3, 'Y', NULL, 1, 93, 'fd', 8238, 6029, 5417, 'TYTOHYYTSn', NULL),
(3, 2, 'FALSE', NULL, 1, 1345, NULL, 88, 386, 210, '9', NULL);
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
(1, 3, 3, NULL, '(voice) (uncredited)', 9288, 2);
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
(1, 3, 1, 1, 'y9v'),
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
(1, 1, 1, 'USA', NULL),
(2, 3, 1, 'USA', NULL);
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
(2, 3, 1, '4.0', 'C7RAAC7ARR'),
(3, 2, 2, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 2);
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
(1, 2, 3, 1),
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
(1, 3, 1, 'j--_j_--j33jWj', NULL);
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
(1, '999999999999999999999999999999', '[us]', NULL, NULL, NULL, NULL),
(2, '', '[de]', NULL, 'W7kwPP3WS7k3wkwk', NULL, 'BBBBB');
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'character-name-in-title', '1e100'),
(3, 'hero-sequel', 'T');
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
(1, 'Other Person', 'B', NULL, NULL, NULL, NULL, NULL, 'i'),
(2, 'Other Person', NULL, 347, '_i_Ii', NULL, '6', NULL, NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, NULL, 'GGGG', 'undefined', 'rEs5orsHxs5Trx');
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
(1, 'Voice Character', NULL, NULL, 'UKxUK', NULL, NULL);
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
(1, 'TRUE', 'K KGk ', 2, 2005, NULL, 'D_DZeeDe_LDZZZDD_De_eDLLD', 393, 6, 236, NULL, NULL),
(2, 'rrr', 'Infinity', 2, NULL, 2609, NULL, NULL, 4504, 3, 'kkB', NULL),
(3, 'KCC', NULL, 1, NULL, 4933, NULL, NULL, 6, 598, 'TPsv%vNv%Pvs', 'v v');
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
(1, 1, 'EE', 'kkkkkk', '-Infinity', 'Wx34x4gW3', NULL, 'ls2'),
(2, 1, 'oFRFF', NULL, NULL, NULL, NULL, 'TRUE');
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
(1, 1, 'if', NULL, 2, NULL, NULL, 2193, NULL, NULL, 'if', 'zHW'),
(2, 3, 'Y', NULL, 1, 93, 'fd', 8238, 6029, 5417, 'TYTOHYYTSn', NULL),
(3, 2, 'FALSE', NULL, 1, 1345, NULL, 88, 386, 210, '9', NULL);
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
(1, 3, 3, NULL, '(voice) (uncredited)', 9288, 2);
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
(1, 3, 1, 1, 'y9v'),
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
(1, 1, 1, 'USA', NULL),
(2, 3, 1, 'USA', NULL);
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
(2, 3, 1, '4.0', 'C7RAAC7ARR'),
(3, 2, 2, '8.0', NULL);
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 2, 2);
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
(1, 2, 3, 1),
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
(1, 3, 1, 'j--_j_--j33jWj', NULL);
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
(1, 'Infinity', '[de]', 5807, NULL, NULL, '8_8_K_');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '7fI17117'),
(2, 'marvel-cinematic-universe', 'qYFqFy');
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
(1, 'Other Person', '00', NULL, NULL, '666666', 'HeeeeeJ', '0', 'HNNHH');
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
(1, 'Voice Character', '', NULL, 'None', 'hhh_k__', NULL),
(2, 'Other Character', 'iiiiiiiiiii', 6, NULL, NULL, 'BS'),
(3, 'Other Character', NULL, NULL, 'null', NULL, 'O6bbb6ObTb');
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
(1, 'uu', 'JJM6M', 1, 2005, NULL, 'f', NULL, NULL, NULL, 'RqRRPD', 'MMwMAwAwAM');
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
(1, 1, '-Infinity', NULL, 'mSm', NULL, '_5h5dd', '3P   '),
(2, 1, 'nil', NULL, NULL, NULL, 'owtEEtw', 'true');
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
(1, 1, 'None', NULL, 1, 428, '', 2283, 6880, 3133, NULL, NULL);
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
(1, 1, 1, 2, '(voice) (uncredited)', 521, 2),
(2, 1, 1, NULL, NULL, NULL, 1),
(3, 1, 1, 2, '(voice)', NULL, 2);
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
(1, 1, 1, 'Bulgaria', 'NUL'),
(2, 1, 2, 'Bulgaria', NULL),
(3, 1, 2, 'Bulgaria', NULL);
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
(1, 1, 2, '4.0', NULL);
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
(1, 1, 2, 'vvvv''MIvI', NULL),
(2, 1, 2, 'SSSSnSnn', NULL);
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
(1, 'Infinity', '[de]', 5807, NULL, NULL, '8_8_K_');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '7fI17117'),
(2, 'marvel-cinematic-universe', 'qYFqFy');
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
(1, 'Other Person', '00', NULL, NULL, '666666', 'HeeeeeJ', '0', 'HNNHH');
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
(1, 'Voice Character', '', NULL, 'None', 'hhh_k__', NULL),
(2, 'Other Character', 'iiiiiiiiiii', 6, NULL, NULL, 'BS'),
(3, 'Other Character', NULL, NULL, 'null', NULL, 'O6bbb6ObTb');
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
(1, 'uu', 'JJM6M', 1, 2005, NULL, 'f', NULL, NULL, NULL, 'RqRRPD', 'MMwMAwAwAM');
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
(1, 1, '-Infinity', NULL, 'mSm', NULL, '_5h5dd', '3P   '),
(2, 1, 'nil', NULL, NULL, NULL, '_5h5dd', 'true');
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
(1, 1, 'None', NULL, 1, 428, '', 2283, 6880, 3133, NULL, NULL);
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
(1, 1, 1, 2, '(voice) (uncredited)', 521, 2),
(2, 1, 1, NULL, NULL, NULL, 1),
(3, 1, 1, 2, '(voice)', NULL, 2);
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
(1, 1, 1, 'Bulgaria', 'NUL'),
(2, 1, 2, 'Bulgaria', NULL),
(3, 1, 2, 'Bulgaria', NULL);
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
(1, 1, 2, '4.0', NULL);
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
(1, 1, 2, 'vvvv''MIvI', NULL),
(2, 1, 2, 'SSSSnSnn', NULL);
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
(1, 'Infinity', NULL, 0, '1', NULL, '8_8_K_');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '7fI17117'),
(2, 'marvel-cinematic-universe', 'qYFqFy');
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
(1, 'Other Person', '00', NULL, NULL, '666666', 'HeeeeeJ', '0', 'HNNHH');
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
(1, 'Voice Character', '', NULL, 'None', 'hhh_k__', NULL),
(2, 'Other Character', 'iiiiiiiiiii', 6, NULL, NULL, 'BS'),
(3, 'Other Character', NULL, NULL, 'null', NULL, 'O6bbb6ObTb');
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
(1, 'uu', 'JJM6M', 1, 2005, NULL, 'f', NULL, NULL, 1, NULL, 'MMwMAwAwAM');
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
(1, 1, '-Infinity', NULL, 'mSm', NULL, '_5h5dd', '3P   '),
(2, 1, 'nil', NULL, NULL, NULL, 'owtEEtw', 'true');
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
(1, 1, 'None', NULL, 1, 428, '', 2283, 6880, 3133, NULL, NULL);
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
(1, 1, 1, 2, '(voice) (uncredited)', 521, 2),
(2, 1, 1, NULL, NULL, NULL, 1),
(3, 1, 1, 2, '(voice)', NULL, 2);
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
(1, 1, 1, 'Bulgaria', 'NUL'),
(2, 1, 2, 'Bulgaria', NULL),
(3, 1, 2, 'Bulgaria', NULL);
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
(1, 1, 2, '4.0', NULL);
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
(1, 1, 2, 'vvvv''MIvI', NULL),
(2, 1, 2, 'SSSSnSnn', NULL);
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
(1, 'Infinity', '[de]', 5807, NULL, NULL, '8_8_K_');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '7fI17117'),
(2, 'marvel-cinematic-universe', 'qYFqFy');
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
(1, 'Other Person', '00', NULL, NULL, '666666', 'HeeeeeJ', '8_8_K_', 'HNNHH');
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
(1, 'Voice Character', '', NULL, 'None', 'hhh_k__', NULL),
(2, 'Other Character', 'iiiiiiiiiii', 6, NULL, NULL, 'BS'),
(3, 'Other Character', NULL, NULL, 'null', NULL, 'O6bbb6ObTb');
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
(1, 'uu', 'JJM6M', 1, 2005, NULL, 'f', NULL, NULL, NULL, 'RqRRPD', 'MMwMAwAwAM');
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
(1, 1, '-Infinity', NULL, 'mSm', NULL, '_5h5dd', '3P   '),
(2, 1, 'nil', NULL, NULL, NULL, 'owtEEtw', 'true');
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
(1, 1, 'None', NULL, 1, 428, '', 2283, 6880, 3133, NULL, NULL);
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
(1, 1, 1, 2, '(voice) (uncredited)', 521, 2),
(2, 1, 1, NULL, NULL, NULL, 1),
(3, 1, 1, 2, '(voice)', NULL, 2);
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
(1, 1, 1, 'Bulgaria', 'NUL'),
(2, 1, 2, 'Bulgaria', NULL),
(3, 1, 2, 'Bulgaria', NULL);
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
(1, 1, 2, '4.0', NULL);
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
(1, 1, 2, 'vvvv''MIvI', NULL),
(2, 1, 2, 'SSSSnSnn', NULL);
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
(1, 'Infinity', '[de]', 5807, NULL, NULL, '8_8_K_');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '7fI17117'),
(2, 'marvel-cinematic-universe', 'qYFqFy');
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
(1, 'Other Person', '00', NULL, NULL, '666666', 'HeeeeeJ', '0', 'HNNHH');
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
(1, 'Voice Character', '', NULL, 'None', 'hhh_k__', NULL),
(2, 'Other Character', 'iiiiiiiiiii', 6, NULL, NULL, 'BS'),
(3, 'Other Character', NULL, NULL, 'null', NULL, 'O6bbb6ObTb');
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
(1, 'uu', 'JJM6M', 1, 2005, NULL, 'f', NULL, NULL, NULL, 'RqRRPD', 'MMwMAwAwAM');
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
(1, 1, '-Infinity', NULL, 'mSm', NULL, '_5h5dd', '3P   '),
(2, 1, 'nil', NULL, NULL, NULL, 'owtEEtw', 'true');
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
(1, 1, 'None', NULL, 1, 428, '', 2283, 6880, 3133, NULL, '1');
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
(1, 1, 1, 2, '(voice)', NULL, 1),
(2, 1, 1, NULL, NULL, NULL, 1),
(3, 1, 1, NULL, '(voice)', NULL, 2);
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
(1, 1, 1, 'Bulgaria', 'NUL'),
(2, 1, 2, 'Bulgaria', NULL),
(3, 1, 2, 'Bulgaria', NULL);
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
(1, 1, 2, '4.0', NULL);
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
(1, 1, 2, 'vvvv''MIvI', NULL),
(2, 1, 2, 'SSSSnSnn', NULL);
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
(1, 'Infinity', '[de]', 5807, NULL, NULL, '8_8_K_');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '7fI17117'),
(2, 'marvel-cinematic-universe', 'qYFqFy');
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
(1, 'Other Person', '00', NULL, NULL, 'MMwMAwAwAM', 'HeeeeeJ', '0', 'HNNHH');
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
(1, 'Voice Character', '', NULL, 'None', 'hhh_k__', NULL),
(2, 'Other Character', 'iiiiiiiiiii', 6, NULL, NULL, 'BS'),
(3, 'Other Character', NULL, NULL, 'null', NULL, 'O6bbb6ObTb');
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
(1, 'uu', 'JJM6M', 1, 2005, NULL, 'f', NULL, NULL, NULL, 'RqRRPD', 'MMwMAwAwAM');
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
(1, 1, '-Infinity', NULL, 'mSm', NULL, '_5h5dd', '3P   '),
(2, 1, 'nil', NULL, NULL, NULL, 'owtEEtw', 'true');
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
(1, 1, 'None', NULL, 1, 428, '', 2283, 6880, 3133, NULL, NULL);
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
(1, 1, 1, 2, '(voice) (uncredited)', 521, 2),
(2, 1, 1, NULL, NULL, NULL, 1),
(3, 1, 1, 2, '(voice)', NULL, 2);
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
(1, 1, 1, 'Bulgaria', 'NUL'),
(2, 1, 2, 'Bulgaria', NULL),
(3, 1, 2, 'Bulgaria', NULL);
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
(1, 1, 2, '4.0', NULL);
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
(1, 1, 2, 'vvvv''MIvI', NULL),
(2, 1, 2, 'SSSSnSnn', NULL);
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
(1, 'TYYTTYYTTYTYYY', NULL, NULL, NULL, NULL, NULL),
(2, 'Ea66zS6a6', '[de]', 3, NULL, '', NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', 'uYJu%o');
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
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'LTMC', 'INF', 'NIL', '0'),
(2, 'Downey Jr., Robert', '', 124, 'TpTpSST', NULL, NULL, NULL, NULL),
(3, 'Other Person', NULL, 1023, '__proto__', '0', 'Frrrr', NULL, NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '-aa'),
(2, 'Voice Character', NULL, NULL, NULL, 'nil', NULL);
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
(1, 'True', NULL, 1, 2012, NULL, NULL, NULL, NULL, NULL, 'Scunthorpe', NULL),
(2, 'DD3', NULL, 1, NULL, NULL, NULL, 2047, NULL, NULL, NULL, 'm');
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
(1, 2, 'G%TI%T%55', 'LL', NULL, 'Inf', NULL, 'Inf');
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
(1, 2, 'jvwv', NULL, 1, NULL, NULL, NULL, 3, NULL, NULL, NULL),
(2, 2, 'G', NULL, 1, NULL, '', 1474, NULL, NULL, NULL, 'NUL');
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
(1, 3, 2, NULL, '(voice)', NULL, 1);
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
(1, NULL, 2, 3);
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
(1, 1, 1, 2, 'none');
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
(2, 1, 1, 'USA', 't'),
(3, 2, 1, 'USA', NULL);
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
(1, 2, 1);
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
(2, 1, 2, 1);
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
(1, 1, 1, 'a8a88818', 'fjE '),
(2, 3, 1, 'null', NULL),
(3, 1, 1, 'LPT1', '6O');
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
(1, 'TYYTTYYTTYTYYY', NULL, NULL, NULL, NULL, NULL),
(2, 'Ea66zS6a6', '[de]', 3, NULL, '', NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', 'uYJu%o');
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
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'LTMC', 'INF', 'NIL', '0'),
(2, 'Downey Jr., Robert', '', 124, 'TpTpSST', NULL, NULL, NULL, NULL),
(3, 'Other Person', NULL, 1023, '__proto__', '-aa', 'Frrrr', NULL, NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '-aa'),
(2, 'Voice Character', NULL, NULL, NULL, 'nil', NULL);
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
(1, 'True', NULL, 1, 2012, NULL, NULL, NULL, NULL, NULL, 'Scunthorpe', NULL),
(2, 'DD3', NULL, 1, NULL, NULL, NULL, 2047, NULL, NULL, NULL, 'm');
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
(1, 2, 'G%TI%T%55', 'LL', NULL, 'Inf', NULL, 'Inf');
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
(1, 2, 'jvwv', NULL, 1, NULL, NULL, NULL, 3, NULL, NULL, NULL),
(2, 2, 'G', NULL, 1, NULL, '', 1474, NULL, NULL, NULL, 'NUL');
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
(1, 3, 2, NULL, '(voice)', NULL, 1);
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
(1, NULL, 2, 3);
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
(1, 1, 1, 2, 'none');
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
(2, 1, 1, 'USA', 't'),
(3, 2, 1, 'USA', NULL);
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
(1, 2, 1);
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
(2, 1, 2, 1);
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
(1, 1, 1, 'a8a88818', 'fjE '),
(2, 3, 1, 'null', NULL),
(3, 1, 1, 'LPT1', '6O');
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
(1, 'TYYTTYYTTYTYYY', NULL, NULL, NULL, NULL, NULL),
(2, 'Ea66zS6a6', '[de]', 3, NULL, '', NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', NULL),
(2, 'hero-sequel', 'uYJu%o');
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
(1, 'Downey Jr., Robert', NULL, NULL, NULL, 'LTMC', 'INF', 'NIL', '0'),
(2, 'Downey Jr., Robert', '', 124, 'TpTpSST', NULL, NULL, NULL, NULL),
(3, 'Other Person', NULL, 1023, '__proto__', '0', 'Frrrr', NULL, NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '-aa'),
(2, 'Voice Character', NULL, NULL, NULL, 'nil', NULL);
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
(1, 'True', NULL, 1, 2012, NULL, NULL, NULL, NULL, NULL, 'Scunthorpe', NULL),
(2, 'DD3', NULL, 1, NULL, NULL, NULL, 2047, NULL, NULL, NULL, 'nil');
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
(1, 2, 'G%TI%T%55', 'LL', NULL, 'Inf', NULL, 'Inf');
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
(1, 2, 'jvwv', NULL, 1, NULL, NULL, NULL, 3, NULL, NULL, NULL),
(2, 2, 'G', NULL, 1, NULL, '', 1474, NULL, NULL, NULL, 'NUL');
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
(1, 3, 2, NULL, '(voice)', NULL, 1);
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
(1, NULL, 2, 3);
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
(1, 1, 1, 2, 'none');
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
(2, 1, 1, 'USA', 't'),
(3, 2, 1, 'USA', NULL);
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
(1, 2, 1);
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
(2, 1, 2, 1);
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
(1, 1, 1, 'a8a88818', 'fjE '),
(2, 3, 1, 'null', NULL),
(3, 1, 1, 'LPT1', '6O');
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
(1, 'ZfZY LZeZ9 Yfg9Leg', '[de]', 1227, '', NULL, 'COM1');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
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
(1, 'references'),
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
(1, 'Downey Jr., Robert', 'r V 8r 88', 4714, NULL, NULL, 'BjjJJJ', NULL, '%H%');
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
(1, 'Other Character', 'N7lNMNdd-7dNee', 511, 'NUL', NULL, 'false'),
(2, 'Voice Character', NULL, NULL, NULL, NULL, NULL),
(3, 'Voice Character', 'S6eP', NULL, NULL, 'llR', NULL);
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
(1, '_OX _X U7ll', NULL, 1, NULL, NULL, NULL, NULL, NULL, 1, NULL, NULL);
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
(1, 1, 'D747oD74o7o4Do', NULL, NULL, 'xFFkDxkFx', '22H', NULL),
(2, 1, 'bMbM', '', 'C3CdGGe33eeCdCeGeGe3CGCGCedCeGe3', NULL, NULL, NULL),
(3, 1, '__proto__', NULL, NULL, 'true', NULL, NULL);
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
(1, 1, 'HHH', NULL, 2, NULL, NULL, 315, NULL, NULL, 'NxNNNx', NULL),
(2, 1, '744q7''', '7K07KK7k', 1, NULL, '_m%QmA ', NULL, 8260, 212, NULL, NULL),
(3, 1, 'MMMMMMMMMMMMMMMMMMMMMM', 'kkoQ', 1, NULL, 'HHaHvs', NULL, 221, NULL, NULL, NULL);
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
(1, 1, 2, 1),
(2, NULL, 1, 2),
(3, 1, 1, 2);
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
(1, 1, 1, 2, 'e8-8WdL8WW8wlWee'),
(2, 1, 1, 1, 'ttttt'),
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
(1, 1, 1, 'USA', 'FALSE'),
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
(1, 1, 1, '8.0', 'FFddxF'),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '4.0', 'ccp9fmcfppf');
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
(1, 1, 1, '1e100', NULL),
(2, 1, 1, '999999999999999999999999999999', NULL);
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
(1, 'ZfZY LZeZ9 Yfg9Leg', '[de]', 1227, '', NULL, 'COM1');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
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
(1, 'references'),
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
(1, 'Downey Jr., Robert', 'r V 8r 88', 4714, NULL, NULL, 'BjjJJJ', NULL, '%H%');
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
(1, 'Other Character', 'N7lNMNdd-7dNee', 511, 'NUL', NULL, 'false'),
(2, 'Voice Character', NULL, NULL, NULL, NULL, NULL),
(3, 'Voice Character', 'S6eP', NULL, NULL, 'llR', NULL);
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
(1, '_OX _X U7ll', NULL, 1, NULL, NULL, NULL, NULL, NULL, 1, NULL, NULL);
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
(1, 1, 'D747oD74o7o4Do', NULL, NULL, 'xFFkDxkFx', '22H', NULL),
(2, 1, 'bMbM', '', 'C3CdGGe33eeCdCeGeGe3CGCGCedCeGe3', NULL, NULL, NULL),
(3, 1, '__proto__', '1', NULL, NULL, NULL, NULL);
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
(1, 1, 'HHH', NULL, 2, NULL, NULL, 315, NULL, NULL, 'NxNNNx', NULL),
(2, 1, '744q7''', '7K07KK7k', 1, NULL, '_m%QmA ', NULL, 8260, 212, NULL, NULL),
(3, 1, 'MMMMMMMMMMMMMMMMMMMMMM', 'kkoQ', 1, NULL, 'HHaHvs', NULL, 221, NULL, NULL, NULL);
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
(1, 1, 2, 1),
(2, NULL, 1, 2),
(3, 1, 1, 2);
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
(1, 1, 1, 2, 'e8-8WdL8WW8wlWee'),
(2, 1, 1, 1, 'ttttt'),
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
(1, 1, 1, 'USA', 'FALSE'),
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
(1, 1, 1, '8.0', 'FFddxF'),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '4.0', 'ccp9fmcfppf');
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
(1, 1, 1, '1e100', NULL),
(2, 1, 1, '999999999999999999999999999999', NULL);
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
(1, 'ZfZY LZeZ9 Yfg9Leg', '[de]', 1227, '', NULL, 'COM1');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
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
(1, 'references'),
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
(1, 'Downey Jr., Robert', 'r V 8r 88', 4714, NULL, NULL, 'BjjJJJ', NULL, '%H%');
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
(1, 'Other Character', 'N7lNMNdd-7dNee', 511, 'NUL', NULL, 'false'),
(2, 'Voice Character', NULL, NULL, NULL, NULL, NULL),
(3, 'Voice Character', 'S6eP', NULL, NULL, 'llR', NULL);
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
(1, '_OX _X U7ll', NULL, 1, NULL, NULL, NULL, NULL, NULL, 1, NULL, NULL);
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
(1, 1, 'D747oD74o7o4Do', NULL, NULL, 'xFFkDxkFx', '22H', NULL),
(2, 1, 'bMbM', '', 'C3CdGGe33eeCdCeGeGe3CGCGCedCeGe3', NULL, NULL, NULL),
(3, 1, '__proto__', NULL, NULL, 'true', NULL, NULL);
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
(1, 1, 'HHH', NULL, 2, NULL, NULL, 315, NULL, NULL, 'NxNNNx', '1'),
(2, 1, '', NULL, 2, NULL, NULL, 1, NULL, NULL, '1', NULL),
(3, 1, '1', NULL, 2, 1, NULL, NULL, NULL, 1, NULL, NULL);
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
(1, NULL, 2, 2),
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
(1, 1, 1, 2, '1'),
(2, 1, 1, 1, NULL),
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
(1, 1, 1, 'USA', NULL),
(2, 1, 1, 'USA', NULL);
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
(1, 1, 1, 2),
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
(1, 1, 1, '1', 'ccp9fmcfppf'),
(2, 1, 1, '1', NULL);
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
(1, 'else', '[ru]', 5429, NULL, NULL, NULL),
(2, 'PcPcc', NULL, 2047, NULL, 'none', ''),
(3, 'QQ''5', '[us]', NULL, 'EE44', 'Inf', NULL);
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
(1, 'marvel-cinematic-universe', '00');
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
(1, 'Other Person', NULL, 1981, 'WO7bW7O3', 'ppk7', NULL, 'cIXTc9XTXI', NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '37PxPU');
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
(1, 'jAAjjn', 'LPT1', 1, 2008, NULL, NULL, NULL, NULL, NULL, 'true', NULL);
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
(1, 1, '_B', NULL, '', NULL, 'HHHMMd', NULL),
(2, 1, 'GP PG', 'COM1', NULL, NULL, NULL, NULL),
(3, 1, 'true', '', 'p444pppp44ppppp44444p4pp44', 'BB8g8qq8qBU', NULL, 'none');
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
(1, 1, 'Tbbs ', NULL, 1, NULL, NULL, NULL, NULL, 511, NULL, NULL),
(2, 1, 'null', 'Z', 2, NULL, '00', NULL, 5090, NULL, '00', NULL);
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
(1, 1, 1, NULL, NULL, NULL, 3),
(2, 1, 1, NULL, NULL, 172, 3),
(3, 1, 1, 1, '(uncredited)', NULL, 2);
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
(2, 1, 1, 2);
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
(1, 1, 2, 1, 'MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM'),
(2, 1, 3, 1, NULL);
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
(1, 1, 1, 'Bulgaria', '%ZcrZc37mZMZr33c337'),
(2, 1, 1, 'Bulgaria', NULL),
(3, 1, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '4.0', 'JZ99'),
(2, 1, 1, '8.0', '''F'''),
(3, 1, 1, '4.0', 'qWcv');
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
(1, 1, 1, 'eyCk7', NULL),
(2, 1, 1, 'else', '');
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
(1, 'else', '[ru]', 5429, NULL, NULL, NULL),
(2, 'PcPcc', NULL, 2047, NULL, 'none', ''),
(3, 'QQ''5', '[us]', NULL, 'EE44', 'Inf', NULL);
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
(1, 'marvel-cinematic-universe', '00');
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
(1, 'Other Person', NULL, 1981, 'WO7bW7O3', 'ppk7', NULL, 'cIXTc9XTXI', NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '37PxPU');
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
(1, 'jAAjjn', 'LPT1', 1, 2008, NULL, NULL, NULL, NULL, NULL, 'true', NULL);
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
(1, 1, '_B', NULL, '', NULL, 'HHHMMd', NULL),
(2, 1, 'GP PG', 'COM1', NULL, NULL, NULL, NULL),
(3, 1, 'true', '', 'p444pppp44ppppp44444p4pp44', 'BB8g8qq8qBU', NULL, 'none');
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
(1, 1, 'Tbbs ', NULL, 1, NULL, NULL, NULL, NULL, 511, NULL, NULL),
(2, 1, 'null', 'Z', 2, NULL, '00', NULL, 5090, NULL, '00', NULL);
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
(1, 1, 1, NULL, NULL, NULL, 3),
(2, 1, 1, NULL, NULL, 172, 3),
(3, 1, 1, 1, '(uncredited)', NULL, 2);
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
(2, 1, 1, 2);
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
(1, 1, 2, 1, 'MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM'),
(2, 1, 3, 1, NULL);
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
(1, 1, 1, 'Bulgaria', '%ZcrZc37mZMZr33c337'),
(2, 1, 1, 'Bulgaria', NULL),
(3, 1, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '4.0', 'JZ99'),
(2, 1, 1, '4.0', '''F'''),
(3, 1, 1, '4.0', 'qWcv');
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
(1, 1, 1, 'eyCk7', NULL),
(2, 1, 1, 'else', '');
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
(1, 'else', '[ru]', 5429, NULL, NULL, NULL),
(2, 'PcPcc', NULL, 2047, NULL, 'none', ''),
(3, 'QQ''5', '[us]', NULL, 'EE44', 'true', NULL);
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
(1, 'marvel-cinematic-universe', '00');
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
(1, 'Other Person', NULL, 1981, 'WO7bW7O3', 'ppk7', NULL, 'cIXTc9XTXI', NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '37PxPU');
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
(1, 'jAAjjn', 'LPT1', 1, 2008, NULL, NULL, NULL, NULL, NULL, 'true', NULL);
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
(1, 1, '_B', NULL, '', NULL, 'HHHMMd', NULL),
(2, 1, 'GP PG', 'COM1', NULL, NULL, NULL, NULL),
(3, 1, 'true', '', 'p444pppp44ppppp44444p4pp44', 'BB8g8qq8qBU', NULL, 'none');
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
(1, 1, 'Tbbs ', NULL, 1, NULL, NULL, NULL, NULL, 511, NULL, NULL),
(2, 1, 'null', 'Z', 2, NULL, '00', NULL, 5090, NULL, '00', NULL);
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
(1, 1, 1, NULL, NULL, NULL, 3),
(2, 1, 1, NULL, NULL, 172, 3),
(3, 1, 1, 1, '(uncredited)', NULL, 2);
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
(2, 1, 1, 2);
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
(1, 1, 2, 1, 'MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM'),
(2, 1, 3, 1, NULL);
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
(1, 1, 1, 'Bulgaria', '%ZcrZc37mZMZr33c337'),
(2, 1, 1, 'Bulgaria', NULL),
(3, 1, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '4.0', 'JZ99'),
(2, 1, 1, '8.0', '''F'''),
(3, 1, 1, '4.0', 'qWcv');
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
(1, 1, 1, 'eyCk7', NULL),
(2, 1, 1, 'else', '');
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
(1, 'else', '[ru]', 5429, NULL, NULL, NULL),
(2, 'PcPcc', NULL, 2047, NULL, 'none', ''),
(3, 'QQ''5', '[us]', NULL, 'EE44', 'Inf', NULL);
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
(1, 'marvel-cinematic-universe', '00');
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
(1, 'Other Person', NULL, 1981, 'WO7bW7O3', 'ppk7', NULL, 'cIXTc9XTXI', NULL);
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
(1, 'Other Character', NULL, NULL, NULL, NULL, '37PxPU');
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
(1, 'jAAjjn', 'LPT1', 1, 2008, NULL, NULL, NULL, NULL, NULL, 'true', NULL);
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
(1, 1, '_B', NULL, '', NULL, 'HHHMMd', NULL),
(2, 1, 'GP PG', 'COM1', NULL, NULL, NULL, NULL),
(3, 1, 'true', '', 'p444pppp44ppppp44444p4pp44', 'BB8g8qq8qBU', NULL, 'none');
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
(1, 1, 'Tbbs ', NULL, 1, NULL, NULL, NULL, NULL, 511, NULL, NULL),
(2, 1, 'null', 'Z', 2, NULL, '00', NULL, 5090, NULL, '00', NULL);
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
(1, 1, 1, NULL, NULL, NULL, 3),
(2, 1, 1, NULL, NULL, 172, 2),
(3, 1, 1, 1, '(uncredited)', NULL, 2);
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
(2, 1, 1, 2);
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
(1, 1, 2, 1, 'MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM'),
(2, 1, 3, 1, NULL);
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
(1, 1, 1, 'Bulgaria', '%ZcrZc37mZMZr33c337'),
(2, 1, 1, 'Bulgaria', NULL),
(3, 1, 1, 'Bulgaria', NULL);
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
(1, 1, 1, '4.0', 'JZ99'),
(2, 1, 1, '8.0', '''F'''),
(3, 1, 1, '4.0', 'qWcv');
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
(1, 1, 1, 'eyCk7', NULL),
(2, 1, 1, 'else', '');
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
(1, 'afLfa', NULL, NULL, NULL, NULL, 'y KF'),
(2, '__proto__', '[us]', 9999, '''t3', 'vvw', NULL),
(3, '-Infinity', '[ru]', 64, NULL, NULL, 'None');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'HHHiiiiHii');
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
(1, 'Downey Jr., Robert', NULL, NULL, 'g', NULL, '', 'LPT1', NULL),
(2, 'Other Person', NULL, 806, '', 'kkoooo', '0AlAl00llA0A0Aeee0lAAeeAel', NULL, 'uJxufjjOJ'),
(3, 'Downey Jr., Robert', NULL, 64, NULL, 'XOO', '', NULL, 'Infinity');
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
(1, 'Other Character', '', 9999, NULL, 'NULL', NULL),
(2, 'Other Character', 'nJ', NULL, NULL, 'Inf', NULL),
(3, 'Voice Character', 'd5JU5h5Xh__33dLhd399JX3X5X5', 3176, NULL, NULL, NULL);
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
(1, '9iii', NULL, 1, 2011, 41, 'xE', NULL, NULL, NULL, '0i0ii33i3030i00ii0000', NULL),
(2, 'O', '__dict__', 1, NULL, 6048, 'fl6f6ZZ6', NULL, 3570, 4, NULL, NULL);
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
(1, 2, 'KsNKu0Kf', NULL, 'nM%', 'IEEWEhIh', 'Y%YGAGD', NULL),
(2, 3, 'if', 'ypypyyyyypp', '', 'GGGdGddvGvGdGdGGGdv', NULL, '11xiy'),
(3, 2, '', NULL, NULL, 'eMMEzM', '%O%', 'LPT1');
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
(1, 1, '', 'BT', 1, 531, NULL, 8191, 8192, 4983, NULL, NULL),
(2, 2, 'none', 'H_h', 1, 55, NULL, 387, 1, 5, NULL, 'FcpcUCFp'),
(3, 2, 'Infinity', 'PPPdfPgdPgfdf', 1, 1037, NULL, 2047, NULL, NULL, 'Cc', 'bOWO');
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
(1, 2, 1, 3, '(voice)', 4095, 2);
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
(1, 2, 1, 1, '-Infinity'),
(2, 2, 2, 1, 'e3SL'),
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
(1, 2, 1, 'USA', NULL),
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
(2, 1, 1, '4.0', 'VWnWVW2'),
(3, 1, 2, '8.0', 'B');
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
(1, 1, 1, 1),
(2, 1, 2, 1),
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
(1, 3, 1, 'NIL', NULL),
(2, 2, 1, '0', 'then');
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
(1, 'afLfa', NULL, NULL, NULL, NULL, 'y KF'),
(2, '__proto__', '[us]', 9999, '''t3', 'vvw', NULL),
(3, '-Infinity', '[ru]', 64, NULL, NULL, 'None');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'HHHiiiiHii');
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
(1, 'Downey Jr., Robert', NULL, NULL, 'g', NULL, '', 'LPT1', NULL),
(2, 'Other Person', NULL, 806, '', 'kkoooo', '0AlAl00llA0A0Aeee0lAAeeAel', NULL, 'uJxufjjOJ'),
(3, 'Downey Jr., Robert', NULL, 64, NULL, 'XOO', '', NULL, 'Infinity');
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
(1, 'Other Character', '', 9999, NULL, 'NULL', NULL),
(2, 'Other Character', 'nJ', NULL, NULL, 'Inf', NULL),
(3, 'Voice Character', 'd5JU5h5Xh__33dLhd399JX3X5X5', 3176, NULL, NULL, NULL);
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
(1, '9iii', NULL, 1, 2011, 41, 'xE', NULL, NULL, NULL, '0i0ii33i3030i00ii0000', NULL),
(2, 'O', '__dict__', 1, NULL, 2, 'fl6f6ZZ6', NULL, 3570, 4, NULL, NULL);
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
(1, 2, 'KsNKu0Kf', NULL, 'nM%', 'IEEWEhIh', 'Y%YGAGD', NULL),
(2, 3, 'if', 'ypypyyyyypp', '', 'GGGdGddvGvGdGdGGGdv', NULL, '11xiy'),
(3, 2, '', NULL, NULL, 'eMMEzM', '%O%', 'LPT1');
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
(1, 1, '', 'BT', 1, 531, NULL, 8191, 8192, 4983, NULL, NULL),
(2, 2, 'none', 'H_h', 1, 55, NULL, 387, 1, 5, NULL, 'FcpcUCFp'),
(3, 2, 'Infinity', 'PPPdfPgdPgfdf', 1, 1037, NULL, 2047, NULL, NULL, 'Cc', 'bOWO');
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
(1, 2, 1, 3, '(voice)', 4095, 2);
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
(1, 2, 1, 1, '-Infinity'),
(2, 2, 2, 1, 'e3SL'),
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
(1, 2, 1, 'USA', NULL),
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
(2, 1, 1, '4.0', 'VWnWVW2'),
(3, 1, 2, '8.0', 'B');
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
(1, 1, 1, 1),
(2, 1, 2, 1),
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
(1, 3, 1, 'NIL', NULL),
(2, 2, 1, '0', 'then');
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
(1, 'afLfa', NULL, NULL, NULL, NULL, 'y KF'),
(2, '__proto__', '[us]', 9999, '''t3', 'vvw', NULL),
(3, '-Infinity', '[ru]', 64, NULL, NULL, 'None');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'HHHiiiiHii');
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
(1, 'Downey Jr., Robert', NULL, NULL, 'g', NULL, '', 'LPT1', NULL),
(2, 'Other Person', NULL, 806, '', 'kkoooo', '0AlAl00llA0A0Aeee0lAAeeAel', NULL, 'uJxufjjOJ'),
(3, 'Downey Jr., Robert', NULL, 64, NULL, 'XOO', '', NULL, 'Infinity');
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
(1, 'Other Character', '', 9999, NULL, 'NULL', NULL),
(2, 'Other Character', 'nJ', NULL, NULL, 'Inf', NULL),
(3, 'Voice Character', 'd5JU5h5Xh__33dLhd399JX3X5X5', 3176, NULL, NULL, NULL);
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
(1, '9iii', NULL, 1, 2011, 41, 'xE', NULL, NULL, NULL, '0i0ii33i3030i00ii0000', NULL),
(2, 'O', '__dict__', 1, NULL, 6048, 'fl6f6ZZ6', NULL, 3570, 4, NULL, NULL);
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
(1, 2, 'KsNKu0Kf', NULL, 'nM%', 'IEEWEhIh', 'Y%YGAGD', NULL),
(2, 3, 'if', 'ypypyyyyypp', '', 'GGGdGddvGvGdGdGGGdv', NULL, '11xiy'),
(3, 2, '', NULL, NULL, 'eMMEzM', '%O%', 'LPT1');
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
(1, 1, '', 'BT', 1, 531, NULL, 8191, 8192, 4983, NULL, NULL),
(2, 2, 'none', 'H_h', 1, 55, NULL, 387, 1, 5, NULL, 'FcpcUCFp'),
(3, 2, 'Infinity', 'PPPdfPgdPgfdf', 1, 1037, NULL, 2047, NULL, NULL, 'Cc', 'bOWO');
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
(1, 2, 1, 3, '(voice)', 4095, 2);
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
(1, 2, 1, 1, '-Infinity'),
(2, 2, 2, 1, 'e3SL'),
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
(1, 2, 1, 'USA', NULL),
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
(2, 1, 1, '4.0', 'VWnWVW2'),
(3, 1, 2, '8.0', 'B');
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
(1, 2, 1, 1),
(2, 1, 2, 1),
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
(1, 3, 1, 'NIL', NULL),
(2, 2, 1, '0', 'then');
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
(1, 'afLfa', NULL, NULL, NULL, NULL, 'y KF'),
(2, '__proto__', '[us]', 9999, '''t3', 'vvw', NULL),
(3, '-Infinity', '[ru]', 64, NULL, NULL, 'None');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'HHHiiiiHii');
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
(1, 'Downey Jr., Robert', NULL, NULL, 'g', NULL, '', 'LPT1', NULL),
(2, 'Other Person', NULL, 806, '', 'kkoooo', '0AlAl00llA0A0Aeee0lAAeeAel', NULL, 'uJxufjjOJ'),
(3, 'Downey Jr., Robert', NULL, 64, NULL, 'XOO', '', NULL, 'Infinity');
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
(1, 'Other Character', '', 9999, NULL, 'NULL', NULL),
(2, 'Other Character', 'nJ', NULL, NULL, 'Inf', NULL),
(3, 'Voice Character', 'd5JU5h5Xh__33dLhd399JX3X5X5', 3176, NULL, NULL, NULL);
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
(1, '9iii', NULL, 1, 2011, 41, 'xE', NULL, NULL, NULL, '0i0ii33i3030i00ii0000', NULL),
(2, 'O', '__dict__', 1, NULL, 6048, 'fl6f6ZZ6', NULL, 3570, 4, NULL, NULL);
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
(1, 2, 'KsNKu0Kf', NULL, 'nM%', 'IEEWEhIh', 'Y%YGAGD', NULL),
(2, 3, 'if', 'ypypyyyyypp', '', 'GGGdGddvGvGdGdGGGdv', NULL, '11xiy'),
(3, 2, '', NULL, NULL, 'eMMEzM', '%O%', 'LPT1');
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
(1, 1, '', 'BT', 1, 531, NULL, 8191, 8192, 4983, NULL, NULL),
(2, 2, 'none', 'H_h', 1, 55, NULL, 387, 1, 5, NULL, 'FcpcUCFp'),
(3, 2, 'Infinity', 'PPPdfPgdPgfdf', 1, 1037, NULL, 2047, NULL, NULL, 'Cc', 'bOWO');
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
(1, 2, 1, 3, '(voice)', 4095, 2);
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
(1, 2, 1, 1, '-Infinity'),
(2, 2, 2, 1, 'e3SL'),
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
(1, 2, 2, '4.0', NULL),
(2, 1, 2, '4.0', NULL),
(3, 1, 1, '4.0', 'VWnWVW2');
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
(1, 2, 2, 1),
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
(1, 1, 2, '1', NULL),
(2, 1, 2, '1', NULL);
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
(1, 'afLfa', NULL, NULL, NULL, NULL, 'y KF'),
(2, '__proto__', '[us]', 9999, '''t3', 'vvw', NULL),
(3, '-Infinity', '[ru]', 64, NULL, NULL, 'None');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'HHHiiiiHii');
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
(1, 'Downey Jr., Robert', NULL, NULL, 'g', NULL, '', 'LPT1', NULL),
(2, 'Other Person', NULL, 806, '', 'kkoooo', '0AlAl00llA0A0Aeee0lAAeeAel', NULL, 'uJxufjjOJ'),
(3, 'Downey Jr., Robert', NULL, 64, NULL, 'XOO', '', NULL, 'Infinity');
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
(1, 'Other Character', '', 9999, NULL, 'NULL', NULL),
(2, 'Other Character', 'nJ', NULL, NULL, 'Inf', NULL),
(3, 'Voice Character', 'd5JU5h5Xh__33dLhd399JX3X5X5', 3176, NULL, NULL, NULL);
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
(1, '9iii', NULL, 1, 2011, 41, 'xE', NULL, NULL, NULL, '0i0ii33i3030i00ii0000', NULL),
(2, 'O', '__dict__', 1, NULL, 6048, 'fl6f6ZZ6', NULL, 3570, 4, NULL, NULL);
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
(1, 2, 'KsNKu0Kf', NULL, 'nM%', 'IEEWEhIh', 'Y%YGAGD', NULL),
(2, 3, 'if', 'ypypyyyyypp', '', 'GGGdGddvGvGdGdGGGdv', NULL, '11xiy'),
(3, 1, '', NULL, NULL, 'eMMEzM', '%O%', 'LPT1');
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
(1, 1, '', 'BT', 1, 531, NULL, 8191, 8192, 4983, NULL, NULL),
(2, 2, 'none', 'H_h', 1, 55, NULL, 387, 1, 5, NULL, 'FcpcUCFp'),
(3, 2, 'Infinity', 'PPPdfPgdPgfdf', 1, 1037, NULL, 2047, NULL, NULL, 'Cc', 'bOWO');
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
(1, 2, 1, 3, '(voice)', 4095, 2);
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
(1, 2, 1, 1, '-Infinity'),
(2, 2, 2, 1, 'e3SL'),
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
(1, 2, 1, 'USA', NULL),
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
(2, 1, 1, '4.0', 'VWnWVW2'),
(3, 1, 2, '8.0', 'B');
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
(1, 1, 1, 1),
(2, 1, 2, 1),
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
(1, 3, 1, 'NIL', NULL),
(2, 2, 1, '0', 'then');
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
(1, 'H2H82FFF8Z', NULL, 1932, NULL, 't', ''),
(2, 'NULL', '[us]', 1903, NULL, NULL, NULL),
(3, 'FZw', NULL, NULL, 'mVVmmVVmmV', NULL, NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '_'),
(2, 'marvel-cinematic-universe', NULL),
(3, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ssssssssssssssssssssssssssssssss', 822, '0', 'KenH%Kn2KHKtnKeHK%2e', NULL, NULL, 'undefined'),
(2, 'Downey Jr., Robert', 'NN', NULL, 'bRRRR', 'GGGG', NULL, 'True', NULL);
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
(1, 'Voice Character', NULL, 8080, NULL, NULL, 'LL9L99l9Lll9ll'),
(2, 'Other Character', 'CYVVYVC', NULL, 'none', NULL, 'EMX'),
(3, 'Voice Character', NULL, NULL, 'TRUE', 'Infinity', 'then');
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
(1, 'gCgCgCg', NULL, 1, 2008, 409, 'ZZtZbc', NULL, NULL, 276, '999999999999999999999999999999', 'yyyJ'),
(2, 'Inf', '', 1, NULL, NULL, NULL, 156, 6979, NULL, NULL, NULL),
(3, '0', NULL, 1, NULL, 90, 'qqqqqqqqqqqqqqqq', 521, NULL, NULL, NULL, 'YYDD');
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
(1, 1, '', '', NULL, NULL, '', NULL);
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
(1, 2, '-', '', 1, 789, NULL, NULL, 268, 925, 'NaN', 'Y');
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
(1, 2, 3, 3, NULL, 324, 1),
(2, 1, 1, NULL, NULL, NULL, 2),
(3, 1, 3, 3, '(uncredited)', NULL, 2);
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
(2, NULL, 2, 1),
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
(1, 2, 2, 'USA', NULL),
(2, 3, 2, 'Bulgaria', 'NNMN'),
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
(1, 3, 1, '8.0', 'LLLLLLXXXLLLLXXXLXLXXX');
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
(1, 1, 3, 1),
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
(1, 1, 1, 'null', NULL);
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
(1, 'H2H82FFF8Z', NULL, 1932, NULL, 't', ''),
(2, 'NULL', '[us]', 1903, NULL, NULL, NULL),
(3, 'FZw', NULL, NULL, 'mVVmmVVmmV', NULL, NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '_'),
(2, 'marvel-cinematic-universe', NULL),
(3, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ssssssssssssssssssssssssssssssss', 822, '0', 'KenH%Kn2KHKtnKeHK%2e', NULL, NULL, 'undefined'),
(2, 'Downey Jr., Robert', 'NN', NULL, 'bRRRR', 'GGGG', NULL, 'True', NULL);
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
(1, 'Voice Character', NULL, 8080, NULL, NULL, 'LL9L99l9Lll9ll'),
(2, 'Other Character', 'CYVVYVC', NULL, 'none', NULL, 'EMX'),
(3, 'Voice Character', NULL, NULL, 'TRUE', 'Infinity', 'then');
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
(1, 'gCgCgCg', NULL, 1, 2008, 409, 'ZZtZbc', NULL, NULL, 276, '999999999999999999999999999999', 'yyyJ'),
(2, 'Inf', '', 1, NULL, NULL, NULL, 156, 6979, NULL, NULL, NULL),
(3, '0', NULL, 1, NULL, 90, 'qqqqqqqqqqqqqqqq', 521, NULL, NULL, NULL, 'YYDD');
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
(1, 1, '', '', NULL, NULL, '', NULL);
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
(1, 2, '-', '', 1, 789, NULL, NULL, 268, 925, 'NaN', 'Y');
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
(1, 2, 3, 3, NULL, 324, 1),
(2, 1, 1, NULL, NULL, NULL, 2),
(3, 1, 2, 3, '(uncredited)', NULL, 2);
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
(2, NULL, 2, 1),
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
(1, 2, 2, 'USA', NULL),
(2, 3, 2, 'Bulgaria', 'NNMN'),
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
(1, 3, 1, '8.0', 'LLLLLLXXXLLLLXXXLXLXXX');
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
(1, 1, 3, 1),
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
(1, 1, 1, 'null', NULL);
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
(1, 'H2H82FFF8Z', NULL, 1932, NULL, 'True', ''),
(2, 'NULL', '[us]', 1903, NULL, NULL, NULL),
(3, 'FZw', NULL, NULL, 'mVVmmVVmmV', NULL, NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '_'),
(2, 'marvel-cinematic-universe', NULL),
(3, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ssssssssssssssssssssssssssssssss', 822, '0', 'KenH%Kn2KHKtnKeHK%2e', NULL, NULL, 'undefined'),
(2, 'Downey Jr., Robert', 'NN', NULL, 'bRRRR', 'GGGG', NULL, 'True', NULL);
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
(1, 'Voice Character', NULL, 8080, NULL, NULL, 'LL9L99l9Lll9ll'),
(2, 'Other Character', 'CYVVYVC', NULL, 'none', NULL, 'EMX'),
(3, 'Voice Character', NULL, NULL, 'TRUE', 'Infinity', 'then');
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
(1, 'gCgCgCg', NULL, 1, 2008, 409, 'ZZtZbc', NULL, NULL, 276, '999999999999999999999999999999', 'yyyJ'),
(2, 'Inf', '', 1, NULL, NULL, NULL, 156, 6979, NULL, NULL, NULL),
(3, '0', NULL, 1, NULL, 90, 'qqqqqqqqqqqqqqqq', 521, NULL, NULL, NULL, 'YYDD');
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
(1, 1, '', '', NULL, NULL, '', NULL);
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
(1, 2, '-', '', 1, 789, NULL, NULL, 268, 925, 'NaN', 'Y');
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
(1, 2, 3, 3, NULL, 324, 1),
(2, 1, 1, NULL, NULL, NULL, 2),
(3, 1, 3, 3, '(uncredited)', NULL, 2);
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
(2, NULL, 2, 1),
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
(1, 2, 2, 'USA', NULL),
(2, 3, 2, 'Bulgaria', 'NNMN'),
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
(1, 3, 1, '8.0', 'LLLLLLXXXLLLLXXXLXLXXX');
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
(1, 1, 3, 1),
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
(1, 1, 1, 'null', NULL);
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
(1, 'H2H82FFF8Z', NULL, 1932, NULL, 't', ''),
(2, 'NULL', '[us]', 1903, NULL, NULL, NULL),
(3, 'FZw', NULL, NULL, 'mVVmmVVmmV', NULL, NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', '_'),
(2, 'marvel-cinematic-universe', NULL),
(3, 'hero-sequel', NULL);
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
(1, 'Downey Jr., Robert', 'ssssssssssssssssssssssssssssssss', 822, '0', 'KenH%Kn2KHKtnKeHK%2e', NULL, NULL, 'undefined'),
(2, 'Downey Jr., Robert', 'CYVVYVC', NULL, 'bRRRR', 'GGGG', NULL, 'True', NULL);
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
(1, 'Voice Character', NULL, 8080, NULL, NULL, 'LL9L99l9Lll9ll'),
(2, 'Other Character', 'CYVVYVC', NULL, 'none', NULL, 'EMX'),
(3, 'Voice Character', NULL, NULL, 'TRUE', 'Infinity', 'then');
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
(1, 'gCgCgCg', NULL, 1, 2008, 409, 'ZZtZbc', NULL, NULL, 276, '999999999999999999999999999999', 'yyyJ'),
(2, 'Inf', '', 1, NULL, NULL, NULL, 156, 6979, NULL, NULL, NULL),
(3, '0', NULL, 1, NULL, 90, 'qqqqqqqqqqqqqqqq', 521, NULL, NULL, NULL, 'YYDD');
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
(1, 1, '', '', NULL, NULL, '', NULL);
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
(1, 2, '-', '', 1, 789, NULL, NULL, 268, 925, 'NaN', 'Y');
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
(1, 2, 3, 3, NULL, 324, 1),
(2, 1, 1, NULL, NULL, NULL, 2),
(3, 1, 3, 3, '(uncredited)', NULL, 2);
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
(2, NULL, 2, 1),
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
(1, 2, 2, 'USA', NULL),
(2, 3, 2, 'Bulgaria', 'NNMN'),
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
(1, 3, 1, '8.0', 'LLLLLLXXXLLLLXXXLXLXXX');
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
(1, 1, 3, 1),
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
(1, 1, 1, 'null', NULL);
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
(1, 'tzzoX9zXX9Xot', '[us]', 3, NULL, NULL, NULL);
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'Z1ZQQ2227Y'),
(2, 'marvel-cinematic-universe', NULL);
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
(1, 'Other Person', 'aa', 2048, 'mhPDPP', NULL, '-Infinity', NULL, NULL),
(2, 'Downey Jr., Robert', 'nn7n', 490, NULL, 'ttUADDD', NULL, 'vEUU', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, '0-', NULL, 'NULL', NULL);
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
(1, 'Voice Character', NULL, 312, NULL, NULL, '-Infinity'),
(2, 'Other Character', 'false', 570, NULL, NULL, 'NIL');
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
(1, 'MxFJDD', 'KK', 1, 2007, NULL, 'NaN', 4096, 3209, 155, NULL, NULL);
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
(1, 1, 'Yek4Fk4ekk', 'sWV-', 'jv', 'false', NULL, NULL),
(2, 2, 'NUL', NULL, '', '7FF7XXXF777XFXvFF', '', NULL),
(3, 2, 'LPT1', NULL, NULL, '00', NULL, 'None');
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
(1, 1, 'N', '_', 1, 2, NULL, 7181, NULL, NULL, NULL, NULL),
(2, 1, '''J''''J6''J''''6', NULL, 1, 86, 't', 68, NULL, 309, NULL, 'v'),
(3, 1, 'SN8NlSDSSlcc8lX', NULL, 1, NULL, NULL, 567, 128, NULL, NULL, 'Jy');
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
(1, 1, 1, 2, NULL, NULL, 1),
(2, 2, 1, 2, NULL, 338, 1),
(3, 1, 1, 1, '(uncredited)', 1106, 1);
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
(1, 1, 1, 2, 'undefined'),
(2, 1, 1, 2, 'none'),
(3, 1, 1, 1, 'E');
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
(2, 1, 1, 'USA', '__proto__');
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
(2, 1, 1, '4.0', 'IIHjHI5'),
(3, 1, 2, '8.0', 'NUL');
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
(1, 2, 3, 'False', NULL);
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
(1, 'tzzoX9zXX9Xot', '[us]', 3, NULL, NULL, NULL);
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'Z1ZQQ2227Y'),
(2, 'marvel-cinematic-universe', NULL);
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
(1, 'Other Person', 'aa', 2048, 'mhPDPP', NULL, '-Infinity', NULL, NULL),
(2, 'Downey Jr., Robert', 'nn7n', 490, NULL, 'ttUADDD', NULL, 'vEUU', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, '0-', NULL, 'NULL', NULL);
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
(1, 'Voice Character', NULL, 312, NULL, NULL, '-Infinity'),
(2, 'Other Character', 'false', 570, NULL, NULL, 'NIL');
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
(1, 'MxFJDD', 'KK', 1, 2007, NULL, 'NaN', 4096, 3209, 155, NULL, NULL);
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
(1, 1, 'Yek4Fk4ekk', 'sWV-', 'jv', 'false', NULL, NULL),
(2, 2, 'NUL', NULL, '', '7FF7XXXF777XFXvFF', '', NULL),
(3, 2, 'LPT1', NULL, '', NULL, NULL, NULL);
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
(1, 1, 'None', NULL, 1, NULL, '_', NULL, 2, NULL, '1', NULL),
(2, 1, '1', NULL, 1, NULL, NULL, 86, 1, NULL, '1', '1'),
(3, 1, 'v', NULL, 1, NULL, NULL, NULL, NULL, NULL, '1', NULL);
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
(1, 2, 1, 2, NULL, NULL, 1),
(2, 2, 1, NULL, NULL, NULL, 2),
(3, 1, 1, NULL, NULL, 338, 1);
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
(1, 1, 1, 1, '1'),
(2, 1, 1, 1, NULL),
(3, 1, 1, 2, 'undefined');
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
(1, 1, 1, 'USA', 'none'),
(2, 1, 1, 'USA', 'E');
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
(2, 1, 1, '4.0', '__proto__'),
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
(1, 1, 1, '1', NULL);
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
(1, 'tzzoX9zXX9Xot', '[us]', 3, NULL, NULL, NULL);
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'Z1ZQQ2227Y'),
(2, 'marvel-cinematic-universe', NULL);
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
(1, 'Other Person', 'aa', 2048, 'mhPDPP', NULL, '-Infinity', NULL, NULL),
(2, 'Downey Jr., Robert', 'nn7n', 490, NULL, 'ttUADDD', NULL, 'vEUU', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, '0-', NULL, 'NULL', NULL);
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
(1, 'Voice Character', NULL, 312, NULL, NULL, '-Infinity'),
(2, 'Other Character', 'false', 570, NULL, NULL, 'NIL');
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
(1, 'MxFJDD', 'KK', 1, 2007, NULL, 'NaN', 4096, 3209, 155, NULL, NULL);
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
(1, 1, 'Yek4Fk4ekk', 'sWV-', 'jv', 'false', NULL, NULL),
(2, 2, 'NUL', NULL, '', '7FF7XXXF777XFXvFF', '', NULL),
(3, 2, 'LPT1', NULL, NULL, '00', NULL, 'None');
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
(1, 1, 'N', '_', 1, 2, NULL, 7181, NULL, NULL, NULL, NULL),
(2, 1, '''J''''J6''J''''6', NULL, 1, 86, 't', 68, NULL, 309, NULL, 'v'),
(3, 1, 'SN8NlSDSSlcc8lX', NULL, 1, NULL, NULL, 567, 128, NULL, NULL, 'Jy');
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
(1, 1, 1, 2, NULL, NULL, 1),
(2, 2, 1, 2, NULL, 338, 1),
(3, 1, 1, 1, '(uncredited)', 1106, 1);
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
(1, 1, 1, 2, 'undefined'),
(2, 1, 1, 2, 'none'),
(3, 1, 1, 1, 'E');
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
(1, 1, 1, 'USA', '1'),
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
(1, 1, 2, '8.0', NULL),
(2, 1, 1, '4.0', 'IIHjHI5'),
(3, 1, 2, '8.0', 'NUL');
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
(1, 2, 3, 'False', NULL);
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
(1, '--s', '[ru]', NULL, 'I%S', NULL, 'FKNNC0'),
(2, 'IQQQQA1', '[ru]', 560, NULL, '', NULL);
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
(1, 'character-name-in-title', 'then'),
(2, 'marvel-cinematic-universe', NULL),
(3, 'marvel-cinematic-universe', NULL);
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
(1, 'references'),
(2, 'follows'),
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
(1, 'Other Person', '', 532, NULL, NULL, '', 'True', 'M6'),
(2, 'Downey Jr., Robert', NULL, 1306, NULL, 'EEEq', NULL, NULL, ''),
(3, 'Downey Jr., Robert', NULL, NULL, '___MM', NULL, NULL, 'E', NULL);
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
(1, 'Other Character', NULL, NULL, 'then', '', 'undefined');
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
(1, 'R', NULL, 1, NULL, 10000, 'TGTGGGS', NULL, 1140, NULL, NULL, NULL);
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
(1, 3, 'None', '999999999999999999999999999999', '-AGJGFJFJ', 'XeB', NULL, NULL),
(2, 3, '', 'ZCzZHz', NULL, NULL, 'then', '');
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
(1, 1, 'UeeYeYee', '0', 1, 720, 'True', NULL, 7105, 4413, 'z1', '');
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
(1, NULL, 2, 1),
(2, NULL, 2, 1),
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
(1, 1, 1, 1, NULL),
(2, 1, 2, 1, 'L  22y 7'),
(3, 1, 1, 1, 'LPT1');
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
(1, 1, 1, 'Bulgaria', 'NUL');
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
(2, 1, 1, '8.0', 'false');
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
(1, 2, 1, '__L_IsL_DLy', 'p-p');
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
(1, '--s', '[ru]', NULL, 'I%S', NULL, 'FKNNC0'),
(2, 'IQQQQA1', '[ru]', 560, NULL, '', NULL);
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
(1, 'character-name-in-title', 'then'),
(2, 'marvel-cinematic-universe', NULL),
(3, 'marvel-cinematic-universe', NULL);
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
(1, 'references'),
(2, 'follows'),
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
(1, 'Other Person', '', 532, NULL, NULL, '', 'True', 'then'),
(2, 'Downey Jr., Robert', NULL, 1306, NULL, 'EEEq', NULL, NULL, ''),
(3, 'Downey Jr., Robert', NULL, NULL, '___MM', NULL, NULL, 'E', NULL);
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
(1, 'Other Character', NULL, NULL, 'then', '', 'undefined');
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
(1, 'R', NULL, 1, NULL, 10000, 'TGTGGGS', NULL, 1140, NULL, NULL, NULL);
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
(1, 3, 'None', '999999999999999999999999999999', '-AGJGFJFJ', 'XeB', NULL, NULL),
(2, 3, '', 'ZCzZHz', NULL, NULL, 'then', '');
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
(1, 1, 'UeeYeYee', '0', 1, 720, 'True', NULL, 7105, 4413, 'z1', '');
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
(1, NULL, 2, 1),
(2, NULL, 2, 1),
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
(1, 1, 1, 1, NULL),
(2, 1, 2, 1, 'L  22y 7'),
(3, 1, 1, 1, 'LPT1');
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
(1, 1, 1, 'Bulgaria', 'NUL');
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
(2, 1, 1, '8.0', 'false');
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
(1, 2, 1, '__L_IsL_DLy', 'p-p');
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
(1, 'complete'),
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
(1, 'RPOPPBBORPPPBBPORRPR', NULL, NULL, NULL, NULL, '3ppu3p-');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'dlz');
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
(1, 'Other Person', NULL, NULL, 'r', '0', NULL, 'na6uu', NULL),
(2, 'Other Person', NULL, 79, NULL, NULL, '33GuG', 'j6G', NULL);
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
(1, 'Other Character', NULL, NULL, 'NaN', 'TRUE', NULL),
(2, 'Voice Character', '0', NULL, NULL, NULL, NULL);
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
(1, 'VtitwV', NULL, 1, NULL, NULL, NULL, 21, 359, NULL, NULL, 'xII1I11Am1''j1x1jA1m1A''Ij');
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
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'false', NULL, 1, NULL, 'FyF''FNN''NI', NULL, NULL, NULL, NULL, '');
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
(1, 1, 1, 1, '(uncredited)', 2047, 1);
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
(2, 1, 1, 1),
(3, 1, 3, 1);
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
(1, 1, 1, 1, 's7r4%7%7%');
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
(1, 1, 1, 'USA', 'BEEnjj'),
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
(1, 1, 1, 3),
(2, 1, 1, 2),
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
(1, 1, 1, '-Infinity', NULL),
(2, 2, 1, 'NUL', 'cccc'),
(3, 2, 1, '0', NULL);
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
(1, 'complete'),
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
(1, 'RPOPPBBORPPPBBPORRPR', NULL, NULL, NULL, NULL, '3ppu3p-');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'dlz');
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
(1, 'Other Person', NULL, NULL, 'r', '0', NULL, 'na6uu', NULL),
(2, 'Other Person', NULL, 79, NULL, NULL, '33GuG', 'j6G', NULL);
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
(1, 'Other Character', NULL, NULL, 'NaN', '-Infinity', NULL),
(2, 'Voice Character', '0', NULL, NULL, NULL, NULL);
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
(1, 'VtitwV', NULL, 1, NULL, NULL, NULL, 21, 359, NULL, NULL, 'xII1I11Am1''j1x1jA1m1A''Ij');
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
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'false', NULL, 1, NULL, 'FyF''FNN''NI', NULL, NULL, NULL, NULL, '');
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
(1, 1, 1, 1, '(uncredited)', 2047, 1);
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
(2, 1, 1, 1),
(3, 1, 3, 1);
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
(1, 1, 1, 1, 's7r4%7%7%');
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
(1, 1, 1, 'USA', 'BEEnjj'),
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
(1, 1, 1, 3),
(2, 1, 1, 2),
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
(1, 1, 1, '-Infinity', NULL),
(2, 2, 1, 'NUL', 'cccc'),
(3, 2, 1, '0', NULL);
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
(1, 'RPOPPBBORPPPBBPORRPR', NULL, NULL, NULL, NULL, '3ppu3p-');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'dlz');
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
(1, 'Other Person', NULL, NULL, 'r', '0', NULL, 'na6uu', NULL),
(2, 'Other Person', NULL, 79, NULL, NULL, '33GuG', 'j6G', NULL);
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
(1, 'Other Character', NULL, NULL, 'NaN', 'TRUE', NULL),
(2, 'Voice Character', '0', NULL, NULL, NULL, NULL);
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
(1, 'VtitwV', NULL, 1, NULL, NULL, NULL, 21, 359, NULL, NULL, 'xII1I11Am1''j1x1jA1m1A''Ij');
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
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'false', NULL, 1, NULL, 'FyF''FNN''NI', NULL, NULL, NULL, NULL, '');
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
(1, 1, 1, 1, '(uncredited)', 2047, 1);
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
(2, 1, 2, 1),
(3, 1, 3, 1);
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
(1, 1, 1, 1, 's7r4%7%7%');
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
(1, 1, 1, 'USA', 'BEEnjj'),
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
(1, 1, 1, 3),
(2, 1, 1, 2),
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
(1, 1, 1, '-Infinity', NULL),
(2, 2, 1, 'NUL', 'cccc'),
(3, 2, 1, '0', NULL);
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
(1, 'RPOPPBBORPPPBBPORRPR', NULL, NULL, NULL, NULL, '3ppu3p-');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'dlz');
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
(1, 'Other Person', NULL, NULL, 'r', '0', NULL, 'na6uu', NULL),
(2, 'Other Person', NULL, 79, NULL, NULL, '33GuG', 'j6G', NULL);
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
(1, 'Other Character', NULL, NULL, 'NaN', 'TRUE', NULL),
(2, 'Voice Character', '0', NULL, NULL, NULL, NULL);
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
(1, 'VtitwV', NULL, 1, NULL, NULL, NULL, 21, 359, NULL, NULL, 'xII1I11Am1''j1x1jA1m1A''Ij');
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
(1, 2, 'else', NULL, NULL, NULL, NULL, NULL);
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
(1, 1, 'false', NULL, 1, NULL, 'FyF''FNN''NI', NULL, NULL, NULL, NULL, '');
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
(1, 1, 1, 1, '(uncredited)', 2047, 1);
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
(2, 1, 1, 1),
(3, 1, 3, 1);
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
(1, 1, 1, 1, 'NUL');
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
(1, 1, 1, 'USA', 'BEEnjj'),
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
(1, 1, 1, 3),
(2, 1, 1, 2),
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
(1, 1, 1, '-Infinity', NULL),
(2, 2, 1, 'NUL', 'cccc'),
(3, 2, 1, '0', NULL);
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
(1, 'OwwwOOKKK7O7K7', '[ru]', NULL, NULL, NULL, NULL),
(2, '1Q5ff', NULL, 4106, NULL, '0Bh0h7h', 'True');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
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
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', NULL);
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
(1, 'Other Person', 'oX-o-', 241, 'HM', 'h', 'hD', NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, '_', 'p', NULL, NULL, 'csscc');
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
(1, 'Voice Character', '0', NULL, NULL, 'NUL', NULL),
(2, 'Other Character', '2u2f22u2ff', 7784, '__proto__', 'nil', 'undefined');
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
(1, 'rJN', '''pb', 2, 2010, 565, 'ipOiO9ppTO', 1597, NULL, 9556, NULL, ''),
(2, 'INF', NULL, 2, NULL, 2197, NULL, 230, 2193, NULL, NULL, NULL);
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
(1, 1, '', NULL, 'if', NULL, NULL, 'ttttttttttttttttt'),
(2, 1, 'A', NULL, NULL, 'W', 'RVAm', NULL);
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
(1, 2, 'X3X3X3X3X333XXXX333X33X', NULL, 1, 2792, 'SkkkkS', NULL, 1024, 306, 'K', 'null'),
(2, 1, '', '00', 1, NULL, 'LPT1', NULL, 135, NULL, NULL, NULL);
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
(1, 2, 1, NULL, '(voice) (uncredited)', 1702, 1),
(2, 2, 2, NULL, '(voice) (uncredited)', NULL, 1),
(3, 1, 1, NULL, NULL, NULL, 1);
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
(1, 1, 2, 1),
(2, 2, 2, 1),
(3, 2, 2, 2);
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
(1, 2, 1, 'USA', 'COM1');
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
(1, 1, 1, '8.0', 'kkkXeX3k3%%'),
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
(1, 2, 1, 3);
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
(1, 2, 1, 'FALSE', NULL);
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
(1, 'OwwwOOKKK7O7K7', '[ru]', NULL, NULL, NULL, NULL),
(2, '1Q5ff', NULL, 4106, NULL, '0Bh0h7h', 'True');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
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
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', NULL);
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
(1, 'Other Person', 'oX-o-', 241, 'HM', 'h', 'hD', NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, '_', 'p', NULL, NULL, 'csscc');
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
(1, 'Voice Character', '0', NULL, NULL, 'NUL', NULL),
(2, 'Other Character', '2u2f22u2ff', 7784, '__proto__', 'nil', 'undefined');
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
(1, 'rJN', 'FALSE', 2, 2010, 565, 'ipOiO9ppTO', 1597, NULL, 9556, NULL, ''),
(2, 'INF', NULL, 2, NULL, 2197, NULL, 230, 2193, NULL, NULL, NULL);
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
(1, 1, '', NULL, 'if', NULL, NULL, 'ttttttttttttttttt'),
(2, 1, 'A', NULL, NULL, 'W', 'RVAm', NULL);
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
(1, 2, 'X3X3X3X3X333XXXX333X33X', NULL, 1, 2792, 'SkkkkS', NULL, 1024, 306, 'K', 'null'),
(2, 1, '', '00', 1, NULL, 'LPT1', NULL, 135, NULL, NULL, NULL);
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
(1, 2, 1, NULL, '(voice) (uncredited)', 1702, 1),
(2, 2, 2, NULL, '(voice) (uncredited)', NULL, 1),
(3, 1, 1, NULL, NULL, NULL, 1);
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
(1, 1, 2, 1),
(2, 2, 2, 1),
(3, 2, 2, 2);
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
(1, 2, 1, 'USA', 'COM1');
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
(1, 1, 1, '8.0', 'kkkXeX3k3%%'),
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
(1, 2, 1, 3);
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
(1, 2, 1, 'FALSE', NULL);
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
(1, 'OwwwOOKKK7O7K7', '[ru]', NULL, NULL, NULL, NULL),
(2, '1Q5ff', NULL, 4106, NULL, '0Bh0h7h', 'True');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
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
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', NULL);
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
(1, 'Other Person', 'oX-o-', 241, 'HM', 'h', 'hD', NULL, NULL),
(2, 'Downey Jr., Robert', NULL, NULL, '_', 'p', NULL, NULL, 'csscc');
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
(1, 'Voice Character', '0', NULL, NULL, 'NUL', NULL),
(2, 'Other Character', '2u2f22u2ff', 7784, '__proto__', 'nil', 'undefined');
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
(1, 'SkkkkS', '''pb', 2, 2010, 565, 'ipOiO9ppTO', 1597, NULL, 9556, NULL, ''),
(2, 'INF', NULL, 2, NULL, 2197, NULL, 230, 2193, NULL, NULL, NULL);
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
(1, 1, '', NULL, 'if', NULL, NULL, 'ttttttttttttttttt'),
(2, 1, 'A', NULL, NULL, 'W', 'RVAm', NULL);
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
(1, 2, 'X3X3X3X3X333XXXX333X33X', NULL, 1, 2792, 'SkkkkS', NULL, 1024, 306, 'K', 'null'),
(2, 1, '', '00', 1, NULL, 'LPT1', NULL, 135, NULL, NULL, NULL);
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
(1, 2, 1, NULL, '(voice) (uncredited)', 1702, 1),
(2, 2, 2, NULL, '(voice) (uncredited)', NULL, 1),
(3, 1, 1, NULL, NULL, NULL, 1);
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
(1, 1, 2, 1),
(2, 2, 2, 1),
(3, 2, 2, 2);
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
(1, 2, 1, 'USA', 'COM1');
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
(1, 1, 1, '8.0', 'kkkXeX3k3%%'),
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
(1, 2, 1, 3);
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
(1, 2, 1, 'FALSE', NULL);
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
(1, '', NULL, NULL, 'True', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', 'U8Uh8U');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode'),
(3, 'episode');
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
(1, 'Other Person', 'UUUUu1u', NULL, NULL, NULL, 'False', 'UfoSffoSoUfUSS', NULL),
(2, 'Other Person', NULL, 128, NULL, NULL, 'r7Ob70rx', 'ggsmgngmsg9gZssmnms', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, 'FALSE', 'TRUE', NULL, 'UdLLUd2xx');
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
(1, 'Other Character', NULL, NULL, NULL, '   ', NULL),
(2, 'Other Character', NULL, 1227, 'h', NULL, '------');
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
(1, 'v', NULL, 2, 2012, 1263, NULL, NULL, NULL, 468, 't', '0s1h'),
(2, 'j', 'IIoAIA6oY66ITToII', 3, NULL, NULL, NULL, 5664, NULL, NULL, NULL, NULL);
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
(1, 3, 'True', '3lgljgg3j', NULL, NULL, NULL, NULL),
(2, 1, 'NNjjqvNvRNccRNN', '00', NULL, 'D', 'GTGWT', '0'),
(3, 1, 'y11gBvvBg9', NULL, NULL, 'Sc_', NULL, NULL);
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
(1, 2, '', 'NUL', 3, NULL, 'null', NULL, NULL, NULL, NULL, 'true'),
(2, 2, 't2gW', 'CXCiiCCXXCCXCCXCXCiiCXC', 1, NULL, NULL, NULL, 409, NULL, NULL, NULL),
(3, 1, 'l5B5g', '', 1, 228, NULL, 5130, 138, NULL, NULL, NULL);
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
(1, 1, 1, NULL, NULL, 5747, 2),
(2, 3, 2, NULL, '(voice)', NULL, 2);
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
(1, 2, 2, 3),
(2, 1, 3, 1);
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
(1, 2, 1, 1, 'sBb');
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
(1, 1, 1, 'USA', 'Zee'),
(2, 2, 1, 'USA', 'Z qS');
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
(1, 2, 1, 3),
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
(1, 2, 1, 'then', '00');
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
(1, '', NULL, NULL, 'True', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', 'U8Uh8U');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode'),
(3, 'episode');
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
(1, 'Other Person', 'UUUUu1u', NULL, NULL, NULL, 'False', 'UfoSffoSoUfUSS', NULL),
(2, 'Other Person', NULL, 128, NULL, NULL, 'r7Ob70rx', 'ggsmgngmsg9gZssmnms', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, 'FALSE', 'TRUE', NULL, 'UdLLUd2xx');
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
(1, 'Other Character', NULL, NULL, NULL, '   ', NULL),
(2, 'Other Character', NULL, 1227, 'h', NULL, '------');
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
(1, 'v', NULL, 2, 2012, 1263, NULL, NULL, NULL, 468, 't', '0s1h'),
(2, 'j', 'IIoAIA6oY66ITToII', 3, NULL, NULL, NULL, 5664, NULL, NULL, NULL, NULL);
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
(1, 3, 'True', '3lgljgg3j', NULL, NULL, NULL, NULL),
(2, 1, 'NNjjqvNvRNccRNN', '00', NULL, 'D', 'GTGWT', '0'),
(3, 1, 'y11gBvvBg9', NULL, NULL, 'Sc_', NULL, NULL);
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
(1, 2, '', 'NUL', 3, NULL, 'null', NULL, NULL, NULL, NULL, 'true'),
(2, 2, 't2gW', 'CXCiiCCXXCCXCCXCXCiiCXC', 1, NULL, NULL, NULL, 409, NULL, NULL, NULL),
(3, 1, 'l5B5g', '', 1, 228, NULL, 5130, 138, NULL, NULL, NULL);
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
(1, 1, 1, NULL, NULL, 1, 2),
(2, 3, 2, NULL, '(voice)', NULL, 2);
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
(1, 2, 2, 3),
(2, 1, 3, 1);
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
(1, 2, 1, 1, 'sBb');
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
(1, 1, 1, 'USA', 'Zee'),
(2, 2, 1, 'USA', 'Z qS');
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
(1, 2, 1, 3),
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
(1, 2, 1, 'then', '00');
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
(1, '', NULL, NULL, 'True', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', 'U8Uh8U');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode'),
(3, 'episode');
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
(1, 'Other Person', 'UUUUu1u', NULL, NULL, NULL, '   ', 'UfoSffoSoUfUSS', NULL),
(2, 'Other Person', NULL, 128, NULL, NULL, 'r7Ob70rx', 'ggsmgngmsg9gZssmnms', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, 'FALSE', 'TRUE', NULL, 'UdLLUd2xx');
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
(1, 'Other Character', NULL, NULL, NULL, '   ', NULL),
(2, 'Other Character', NULL, 1227, 'h', NULL, '------');
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
(1, 'v', NULL, 2, 2012, 1263, NULL, NULL, NULL, 468, 't', '0s1h'),
(2, 'j', 'IIoAIA6oY66ITToII', 3, NULL, NULL, NULL, 5664, NULL, NULL, NULL, NULL);
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
(1, 3, 'True', '3lgljgg3j', NULL, NULL, NULL, NULL),
(2, 1, 'NNjjqvNvRNccRNN', '00', NULL, 'D', 'GTGWT', '0'),
(3, 1, 'y11gBvvBg9', NULL, NULL, 'Sc_', NULL, NULL);
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
(1, 2, '', 'NUL', 3, NULL, 'null', NULL, NULL, NULL, NULL, 'true'),
(2, 2, 't2gW', 'CXCiiCCXXCCXCCXCXCiiCXC', 1, NULL, NULL, NULL, 409, NULL, NULL, NULL),
(3, 1, 'l5B5g', '', 1, 228, NULL, 5130, 138, NULL, NULL, NULL);
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
(1, 1, 1, NULL, NULL, 5747, 2),
(2, 3, 2, NULL, '(voice)', NULL, 2);
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
(1, 2, 2, 3),
(2, 1, 3, 1);
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
(1, 2, 1, 1, 'sBb');
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
(1, 1, 1, 'USA', 'Zee'),
(2, 2, 1, 'USA', 'Z qS');
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
(1, 2, 1, 3),
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
(1, 2, 1, 'then', '00');
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
(1, '', NULL, NULL, 'True', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', 'U8Uh8U');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode'),
(3, 'episode');
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
(1, 'Other Person', 'UUUUu1u', NULL, NULL, NULL, 'False', 'UfoSffoSoUfUSS', NULL),
(2, 'Other Person', NULL, 128, NULL, NULL, 'r7Ob70rx', 'ggsmgngmsg9gZssmnms', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, 'FALSE', 'TRUE', NULL, 'UdLLUd2xx');
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
(1, 'Other Character', NULL, NULL, NULL, '   ', NULL),
(2, 'Other Character', NULL, 1227, 'h', NULL, '------');
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
(1, 'v', NULL, 2, 2012, 1263, NULL, NULL, NULL, 468, 't', '0s1h'),
(2, 'j', 'IIoAIA6oY66ITToII', 3, NULL, NULL, NULL, 5664, NULL, NULL, NULL, NULL);
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
(1, 3, 'True', '3lgljgg3j', NULL, NULL, NULL, NULL),
(2, 1, 'NNjjqvNvRNccRNN', NULL, NULL, NULL, NULL, 'D'),
(3, 2, 'GTGWT', '0', NULL, NULL, NULL, NULL);
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
(1, 2, '', NULL, 2, NULL, NULL, NULL, 1, NULL, NULL, NULL),
(2, 2, '', NULL, 2, NULL, NULL, NULL, NULL, 1, NULL, NULL),
(3, 2, '1', NULL, 2, NULL, NULL, NULL, NULL, 409, NULL, NULL);
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
(1, 2, 1, NULL, NULL, NULL, 2),
(2, 2, 1, 2, NULL, 5130, 2);
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
(1, 2, 1, 2, NULL);
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
(1, 1, 1, 2),
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
(1, 1, 1, '', NULL);
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
(1, '', NULL, NULL, 'True', NULL, NULL);
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', 'U8Uh8U');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'episode'),
(2, 'episode'),
(3, 'episode');
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
(1, 'Other Person', 'UUUUu1u', NULL, NULL, NULL, 'False', 'UfoSffoSoUfUSS', NULL),
(2, 'Other Person', NULL, 128, NULL, NULL, 'r7Ob70rx', 'ggsmgngmsg9gZssmnms', NULL),
(3, 'Downey Jr., Robert', NULL, NULL, NULL, 'FALSE', 'TRUE', NULL, 'UdLLUd2xx');
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
(1, 'Other Character', NULL, NULL, NULL, '   ', NULL),
(2, 'Other Character', NULL, 1227, 'h', NULL, '------');
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
(1, 'v', NULL, 2, 2012, 1263, NULL, NULL, NULL, 468, 't', '0s1h'),
(2, 'j', 'IIoAIA6oY66ITToII', 3, NULL, NULL, NULL, 5664, NULL, NULL, NULL, NULL);
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
(1, 3, 'True', '3lgljgg3j', NULL, NULL, NULL, NULL),
(2, 1, 'NNjjqvNvRNccRNN', '00', NULL, 'D', 'GTGWT', '0'),
(3, 1, 'y11gBvvBg9', NULL, NULL, 'Sc_', NULL, NULL);
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
(1, 2, '', 'NUL', 3, NULL, 'null', NULL, NULL, NULL, NULL, 'true'),
(2, 2, 't2gW', 'CXCiiCCXXCCXCCXCXCiiCXC', 1, NULL, NULL, NULL, 409, NULL, NULL, NULL),
(3, 1, 'l5B5g', '', 1, 228, NULL, 5130, 138, NULL, NULL, NULL);
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
(1, 1, 1, NULL, NULL, 5747, 2),
(2, 3, 2, NULL, '(voice)', NULL, 2);
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
(1, 2, 2, 3),
(2, 1, 3, 1);
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
(1, 2, 1, 1, 'sBb');
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
(1, 1, 1, 'USA', 'Zee'),
(2, 1, 1, 'USA', 'Z qS');
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
(1, 2, 1, 3),
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
(1, 2, 1, 'then', '00');
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
(1, '', '[ru]', 934, NULL, 'IIIILII''4II%L', 'TppddkkdkdppdTTdddkddkdpkpT'),
(2, 'false', '[de]', NULL, NULL, NULL, NULL);
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
(3, 'rating');
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
(1, 'Other Person', '', 236, NULL, 'then', NULL, NULL, 'ttYYt'),
(2, 'Other Person', 'ZLb', NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Voice Character', '', 104, NULL, NULL, 'Scunthorpe'),
(2, 'Voice Character', 'L', 4095, NULL, 'None', 'y');
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
(1, 'cccccc', NULL, 1, 2011, NULL, NULL, 1000, NULL, 511, 'h', NULL),
(2, 'False', NULL, 1, 2010, NULL, 'bbX''bmIXmbbX''', 8271, NULL, 4887, '', 'Scunthorpe');
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
(1, 2, 'BB', 'uuuu', NULL, 'False', 'php', 'False'),
(2, 1, '7KKf', NULL, NULL, 'AV', '0', NULL),
(3, 1, 'Inf', '''7XMMf''MjX', '1e100', 'else', 'I', '');
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
(1, 1, 'NULL', '1m1mmm1m11mm11m1', 1, 23, '1e100', NULL, 1202, 16, 'pR', ''),
(2, 1, '', NULL, 1, NULL, '00', NULL, 128, NULL, NULL, NULL),
(3, 2, 'true', 'yyyyyyyyyyyyyy', 1, NULL, NULL, 530, NULL, NULL, 'then', NULL);
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
(1, 2, 1, 1, NULL, NULL, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 2022, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', 20, 3);
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
(1, NULL, 1, 3);
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
(1, 2, 1, 'USA', 'oooo'),
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
(1, 1, 2, '8.0', ''),
(2, 2, 3, '8.0', 'True'),
(3, 2, 1, '8.0', 'HqHHH');
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
(1, 1, 2, 1),
(2, 2, 1, 1),
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
(1, 2, 1, '', '-Infinity');
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
(1, '', '[ru]', 934, NULL, 'IIIILII''4II%L', 'TppddkkdkdppdTTdddkddkdpkpT'),
(2, 'false', '[de]', NULL, NULL, NULL, NULL);
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
(3, 'rating');
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
(1, 'Other Person', '', 236, NULL, 'y', NULL, NULL, 'ttYYt'),
(2, 'Other Person', 'ZLb', NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Voice Character', '', 104, NULL, NULL, 'Scunthorpe'),
(2, 'Voice Character', 'L', 4095, NULL, 'None', 'y');
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
(1, 'cccccc', NULL, 1, 2011, NULL, NULL, 1000, NULL, 511, 'h', NULL),
(2, 'False', NULL, 1, 2010, NULL, 'bbX''bmIXmbbX''', 8271, NULL, 4887, '', 'Scunthorpe');
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
(1, 2, 'BB', 'uuuu', NULL, 'False', 'php', 'False'),
(2, 1, '7KKf', NULL, NULL, 'AV', '0', NULL),
(3, 1, 'Inf', '''7XMMf''MjX', '1e100', 'else', 'I', '');
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
(1, 1, 'NULL', '1m1mmm1m11mm11m1', 1, 23, '1e100', NULL, 1202, 16, 'pR', ''),
(2, 1, '', NULL, 1, NULL, '00', NULL, 128, NULL, NULL, NULL),
(3, 2, 'true', 'yyyyyyyyyyyyyy', 1, NULL, NULL, 530, NULL, NULL, 'then', NULL);
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
(1, 2, 1, 1, NULL, NULL, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 2022, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', 20, 3);
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
(1, NULL, 1, 3);
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
(1, 2, 1, 'USA', 'oooo'),
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
(1, 1, 2, '8.0', ''),
(2, 2, 3, '8.0', 'True'),
(3, 2, 1, '8.0', 'HqHHH');
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
(1, 1, 2, 1),
(2, 2, 1, 1),
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
(1, 2, 1, '', '-Infinity');
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
(1, '', '[ru]', 934, NULL, 'IIIILII''4II%L', 'TppddkkdkdppdTTdddkddkdpkpT'),
(2, 'false', '[de]', NULL, NULL, NULL, NULL);
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
(3, 'rating');
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
(1, 'Other Person', '', 236, NULL, 'then', NULL, NULL, 'ttYYt'),
(2, 'Other Person', 'ZLb', NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Voice Character', '', 104, NULL, NULL, 'Scunthorpe'),
(2, 'Voice Character', 'L', 4095, NULL, 'None', 'y');
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
(1, 'cccccc', NULL, 1, 2011, NULL, NULL, 1000, NULL, 511, 'h', NULL),
(2, 'False', NULL, 1, 2010, NULL, 'bbX''bmIXmbbX''', 8271, NULL, 4887, '', 'Scunthorpe');
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
(1, 2, 'BB', 'uuuu', NULL, 'False', 'php', 'False'),
(2, 1, '7KKf', NULL, NULL, 'AV', '0', NULL),
(3, 1, 'Inf', '''7XMMf''MjX', '1e100', 'else', 'I', '');
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
(1, 1, 'NULL', '1m1mmm1m11mm11m1', 1, 23, '1e100', NULL, 1202, 16, 'pR', ''),
(2, 1, '', NULL, 1, NULL, '00', NULL, 128, NULL, NULL, NULL),
(3, 2, 'true', 'yyyyyyyyyyyyyy', 1, NULL, NULL, 530, NULL, NULL, 'then', NULL);
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
(1, 2, 1, 1, NULL, NULL, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 2022, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', 20, 3);
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
(1, NULL, 1, 3);
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
(1, 2, 1, 'USA', 'true'),
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
(1, 1, 2, '8.0', ''),
(2, 2, 3, '8.0', 'True'),
(3, 2, 1, '8.0', 'HqHHH');
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
(1, 1, 2, 1),
(2, 2, 1, 1),
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
(1, 2, 1, '', '-Infinity');
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
(1, '', '[ru]', 934, NULL, 'IIIILII''4II%L', 'TppddkkdkdppdTTdddkddkdpkpT'),
(2, 'false', '[de]', NULL, NULL, NULL, NULL);
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
(3, 'rating');
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
(1, 'Other Person', '', 236, NULL, 'then', NULL, NULL, 'ttYYt'),
(2, 'Other Person', 'ZLb', NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Voice Character', '', 0, NULL, NULL, 'Scunthorpe'),
(2, 'Voice Character', 'L', 4095, NULL, 'None', 'y');
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
(1, 'cccccc', NULL, 1, 2011, NULL, NULL, 1000, NULL, 511, 'h', NULL),
(2, 'False', NULL, 1, 2010, NULL, 'bbX''bmIXmbbX''', 8271, NULL, 4887, '', 'Scunthorpe');
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
(1, 2, 'BB', 'uuuu', NULL, 'False', 'php', 'False'),
(2, 1, '7KKf', NULL, NULL, 'AV', '0', NULL),
(3, 1, 'Inf', '''7XMMf''MjX', '1e100', 'else', 'I', '');
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
(1, 1, 'NULL', '1m1mmm1m11mm11m1', 1, 23, '1e100', NULL, 1202, 16, 'pR', ''),
(2, 1, '', NULL, 1, NULL, '00', NULL, 128, NULL, NULL, NULL),
(3, 2, 'true', 'yyyyyyyyyyyyyy', 1, NULL, NULL, 530, NULL, NULL, 'then', NULL);
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
(1, 2, 1, 1, NULL, NULL, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 2022, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', 20, 3);
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
(1, NULL, 1, 3);
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
(1, 2, 1, 'USA', 'oooo'),
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
(1, 1, 2, '8.0', ''),
(2, 2, 3, '8.0', 'True'),
(3, 2, 1, '8.0', 'HqHHH');
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
(1, 1, 2, 1),
(2, 2, 1, 1),
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
(1, 2, 1, '', '-Infinity');
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
(1, '', '[ru]', 934, NULL, 'IIIILII''4II%L', 'TppddkkdkdppdTTdddkddkdpkpT'),
(2, 'false', '[de]', NULL, NULL, NULL, NULL);
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
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '1');
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
(1, 'Other Person', '1', 1, NULL, NULL, NULL, 'ttYYt', NULL),
(2, 'Other Person', NULL, NULL, NULL, NULL, NULL, NULL, NULL);
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
(1, 'Other Character', NULL, 1, '1', NULL, 'Scunthorpe'),
(2, 'Voice Character', 'L', 4095, NULL, 'None', 'y');
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
(1, 'cccccc', NULL, 1, 2011, NULL, NULL, 1000, NULL, 511, 'h', NULL),
(2, 'False', NULL, 1, 2010, NULL, 'bbX''bmIXmbbX''', 8271, NULL, 4887, '', 'Scunthorpe');
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
(1, 2, 'BB', 'uuuu', NULL, 'False', 'php', 'False'),
(2, 1, '7KKf', NULL, NULL, 'AV', '0', NULL),
(3, 1, 'Inf', '''7XMMf''MjX', '1e100', 'else', 'I', '');
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
(1, 1, 'NULL', '1m1mmm1m11mm11m1', 1, 23, '1e100', NULL, 1202, 16, 'pR', ''),
(2, 1, '', NULL, 1, NULL, '00', NULL, 128, NULL, NULL, NULL),
(3, 2, 'true', 'yyyyyyyyyyyyyy', 1, NULL, NULL, 530, NULL, NULL, 'then', NULL);
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
(1, 2, 1, 1, NULL, NULL, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 2022, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', 20, 3);
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
(1, NULL, 1, 3);
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
(1, 2, 1, 'USA', 'oooo'),
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
(1, 1, 2, '8.0', ''),
(2, 2, 3, '8.0', 'True'),
(3, 2, 1, '8.0', 'HqHHH');
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
(1, 1, 2, 1),
(2, 2, 1, 1),
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
(1, 2, 1, '', '-Infinity');
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
(1, '', '[ru]', 934, NULL, 'IIIILII''4II%L', 'TppddkkdkdppdTTdddkddkdpkpT'),
(2, 'false', '[de]', NULL, NULL, NULL, NULL);
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
(3, 'rating');
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
(1, 'Other Person', '', 236, NULL, 'then', NULL, NULL, 'ttYYt'),
(2, 'Other Person', 'ZLb', NULL, NULL, NULL, NULL, NULL, NULL);
CREATE TABLE role_type (
  id BIGINT NOT NULL,
  role VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO role_type VALUES
(1, 'actress'),
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
(1, 'Voice Character', '', 104, NULL, NULL, 'Scunthorpe'),
(2, 'Voice Character', 'L', 4095, NULL, 'None', 'y');
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
(1, 'cccccc', NULL, 1, 2011, NULL, NULL, 1000, NULL, 511, 'h', NULL),
(2, 'False', NULL, 1, 2010, NULL, 'bbX''bmIXmbbX''', 8271, NULL, 4887, '', 'Scunthorpe');
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
(1, 2, 'BB', 'uuuu', NULL, 'False', 'php', 'False'),
(2, 1, '7KKf', NULL, NULL, 'AV', '0', NULL),
(3, 1, 'Inf', '''7XMMf''MjX', '1e100', 'else', 'I', '');
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
(1, 1, 'NULL', '1m1mmm1m11mm11m1', 1, 23, '1e100', NULL, 1202, 16, 'pR', ''),
(2, 1, '', NULL, 1, NULL, '00', NULL, 128, NULL, NULL, NULL),
(3, 2, 'true', 'yyyyyyyyyyyyyy', 1, NULL, NULL, 530, NULL, NULL, 'then', NULL);
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
(1, 2, 1, 1, NULL, NULL, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 2022, 2),
(3, 2, 1, NULL, '(voice) (uncredited)', 20, 3);
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
(1, NULL, 1, 3);
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
(1, 2, 1, 'USA', 'oooo'),
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
(1, 1, 2, '8.0', ''),
(2, 2, 3, '8.0', 'True'),
(3, 2, 1, '8.0', 'HqHHH');
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
(1, 1, 1, 1),
(2, 2, 1, 1),
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
(1, 2, 1, '', '-Infinity');
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
(1, 'False', '[de]', 312, '00CCC00CC', 'xxxxxQxxQxQx', NULL),
(2, '0', '[ru]', NULL, 'NULL', NULL, 'xb');
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
(1, 'episode');
CREATE TABLE link_type (
  id BIGINT NOT NULL,
  link VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO link_type VALUES
(1, 'follows'),
(2, 'follows'),
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
(1, 'Other Person', NULL, NULL, '', NULL, NULL, NULL, 'B''sbT6T');
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
(1, 'Other Character', 'ii', 3715, 'TS', NULL, 'mnm'),
(2, 'Other Character', NULL, 23, NULL, '8', 'gGgggGG');
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
(1, 'True', '', 1, NULL, NULL, NULL, NULL, NULL, 548, NULL, NULL),
(2, 'ccc6Z85Z5', NULL, 1, NULL, NULL, NULL, 144, NULL, 9999, NULL, 'P');
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
(1, 1, 'MDDMqq8Zg', 'kmmk4k', 'x', NULL, '''xuSx''', NULL),
(2, 1, 'zy_naa_3cz8z', NULL, NULL, 'COM1', NULL, 'none'),
(3, 1, 'v', 'null', 't3C3tC3ZlklZkCllkc', NULL, 'EEeJ''HEeeeeeCH''eCe''HJJeJH''eCEJ', NULL);
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
(1, 2, 'r', NULL, 1, NULL, 'a2zzaT', NULL, NULL, 409, '', NULL),
(2, 1, 'WWOfWWOA', NULL, 1, NULL, NULL, NULL, NULL, NULL, NULL, 'd');
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
(1, 1, 2, 1, '(uncredited)', 1680, 2),
(2, 1, 1, 2, '(uncredited)', 200, 2);
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
(1, 1, 1, 1, NULL),
(2, 1, 1, 1, 'FALSE');
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
(1, 2, 2, 'Bulgaria', NULL);
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
(1, 1, 3, '4.0', 'Scunthorpe');
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
(1, 2, 2, 3),
(2, 2, 2, 3);
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
(1, 1, 1, '', NULL);
ROLLBACK;

