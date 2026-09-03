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
(1, 'YYYY', NULL, 511, '', '-Infinity', 'UMUr'),
(2, 'NNNNNNNNNNNNNNNNN', '[us]', 3, 'SS', 'y', '4-'),
(3, 'E', '[us]', 2048, '', 'R', 'false');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', '9999999'),
(2, 'character-name-in-title', '-Infinity');
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
(1, 'Downey Jr., Robert', NULL, 6, '3qqJq33', 'XSKKSX8ZXO', NULL, 'none', 'EES'),
(2, 'Downey Jr., Robert', '8gcgc', 1669, 'TTQQH33H33H3HT333', 'n', '2xyxx2BxxeyBexx', 'eb4', 'lp');
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
(1, 'Voice Character', 'JJ4', 759, '%%%%%%%%%%%%%%%%%%%%%%%%%%%', 'M6', 'LPT1'),
(2, 'Voice Character', 'hlc', 511, '-nn--QQ', '-Infinity', 'KeR'),
(3, 'Other Character', 'zXjzrcbjjz3', 4595, 'undefined', 'NK', 'O--Jiisi');
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
(1, '1f', 'TTTTTTTT', 1, 2011, 443, 'INF', 8192, 8191, 3769, 'S', 'true');
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
(1, 2, 'b', NULL, 'JpffcfJ', '3Zv389T3Z3', '', 'True');
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
(1, 1, 'wZZwZZ', 'II', 1, 511, '00', 16, 0, 5, 'PP3fmf93oRf9RoRmf', '3i'),
(2, 1, '00', '3XL3rXrLLa', 1, 256, 'UU98', 357, 208, 5583, 'Wrr', 'xJxxxL'),
(3, 1, '995n', 'WWy', 1, 285, 'WWL', 1024, 7, 4095, 'uauuaPPPuPPPfau', 'False');
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
(1, 1, 1, 2, '(uncredited)', 1289, 1),
(2, 1, 1, 2, '(uncredited)', 512, 2),
(3, 1, 1, 2, '(voice)', 15, 1);
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
(1, 1, 1, 1, 'uydddyHyd1d');
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
(1, 1, 1, '4.0', 'r'),
(2, 1, 2, '4.0', 'G      G G   G G GGG G  ');
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
(1, 1, 1, 3),
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
(1, 2, 1, 'HgcqggHHccqH', 'zccLz1dzcdddz'),
(2, 2, 2, 'Infinity', 'euoyeay'),
(3, 1, 1, 'OOOS', 'ui');
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
(1, '_viy2MM_uM', '[ru]', 41, NULL, 'WPsPQ55us', '-Y-Y'),
(2, 'f', '[de]', 482, '0', '', NULL),
(3, 'iQnQnninn', '[us]', 9280, 'null', 'TB', 's');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'None');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie'),
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
(1, 'Other Person', 'then', 299, NULL, NULL, NULL, 'ywwyywyWDw', NULL),
(2, 'Other Person', 'coc', 225, '0', 'EEEEEEEEEEEE', 'COM1', '999999999999999999999999999999', '1u1au'),
(3, 'Downey Jr., Robert', 'TRUE', NULL, 'atta%ttt', NULL, 'LPT1', 'None', '6AAAB66666B6BB66BAB');
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
(1, 'Voice Character', '', 1596, '0070770777070', 'Y6O5', '__proto__');
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
(1, 'NIL', '', 2, NULL, 1311, '', 562, NULL, 32, 'r', '___x__xx_xx_x____');
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
(1, 3, 'OqOON', 'nnnQnc', NULL, 'VphVVVhhhpph', 'R_IR57kky77_5yY', 'aGvauSGS-SKvK');
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
(1, 1, 'df''', 'p1Q', 1, 9654, NULL, NULL, 2180, 38, '33l', 'gW'),
(2, 1, '', '', 2, 1947, 'cazadNdNczVzNNz', 748, 739, 969, 'CCCCC', 'uq'),
(3, 1, 'ki', 'None', 1, NULL, 'l', NULL, 604, NULL, '0YpppSpSSS0', '_');
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
(1, 2, 1, 1, '(voice)', 3888, 2);
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
(1, 1, 2, 1);
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
(1, 1, 2, 2, 'YxYxYYYYYYxxYYYYxxxxxxYxYxY'),
(2, 1, 2, 1, '-Infinity');
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
(1, 1, 2, 'USA', '');
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
(1, 1, 1, '8.0', 'Szqqhqq'),
(2, 1, 2, '8.0', 'ODNy1');
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
(1, 2, 2, 'Tdoo', '-Infinity');
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
(1, 'null', '[us]', NULL, '', '', 'TRUE');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'True'),
(2, 'character-name-in-title', NULL),
(3, 'character-name-in-title', 'else');
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
(1, 'Other Person', 'JB4adrdrJ9a9', 2328, '5L5', 'true', '4nn', '', 'wwwwwwwwwwwwww'),
(2, 'Downey Jr., Robert', '', NULL, 'COM1', 'undefined', 'CYYCtYCCCtCYCtCCYC', 'BBByEE', 'undefined');
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
(1, 'Voice Character', 'WW', 6690, 'J', 'j6a44UU6', ''),
(2, 'Voice Character', '00', 2559, 'jt', 'COM1', 'FALSE');
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
(1, 'BPe44BBe', 'Ef', 2, 2009, 32, '999999999999999999999999999999', 0, 196, 80, '0', ''),
(2, '%8Q%-%''%%%''%%l', '5_6', 1, 2005, 3851, NULL, 507, 2, 110, NULL, 'jEbTVj');
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
(1, 2, '1hIi', '99vzz9', 'vv777vvvv''', '__proto__', '__proto__', 'true'),
(2, 2, 'null', 'L4', '--L', 'm-m339W9', 'U', 'if');
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
(1, 1, 'Lu', NULL, 2, 41, '33v33fGs3', NULL, 288, 382, 'v8vvi8vii', 'DDYSYDYYSDYSSY'),
(2, 1, 'ww', 'NaN', 1, 8677, '777xx7', 2846, 1213, 2331, '', 'vvYYavv');
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
(1, 1, 1, 2, '(voice)', 344, 1);
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
(1, 2, 3, 1);
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
(1, 1, 1, 1, 'jQ888');
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
(1, 1, 1, 'Bulgaria', '1'),
(2, 2, 1, 'USA', '9');
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
(1, 2, 1, '8.0', 'else'),
(2, 1, 1, '8.0', 'k5'),
(3, 2, 1, '4.0', 'ylGL');
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
(1, 2, 1, 2),
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
(1, 2, 1, 'BkE0', 'None');
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
(1, 'NULL', '[ru]', NULL, '00', NULL, 'g5ggKpKpgzK2');
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
(1, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'PaW');
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
(1, 'Other Person', NULL, 511, '7', 'JJZJJZJGJJGZZGZZZZZGGGJG', '1e100', '', '8-8-a--m--'),
(2, 'Other Person', 'b', 9999, 'NaN', 'then', '', 'XRX5R5R', 'True');
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
(1, 'Voice Character', 'false', 0, 'dkb', '', '1m'),
(2, 'Voice Character', '00', 512, NULL, 'then', 'c77777ccc7');
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
(1, '778887887', '-S', 1, 2007, 16, 'S', 1024, 219, 31, 'if', 'bta'),
(2, '8', '66YY6YY', 1, 2010, 660, 'G_s', NULL, 1548, 276, '00', '6X');
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
(1, 2, '', 'Infinity', 'h', 'XIIXX', 'qwqqqqqqq', 'true'),
(2, 2, '', '00', '9', NULL, 's%s', 'fcfrffcqrc');
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
(1, 2, '0', NULL, 1, 525, NULL, 31, 7028, 184, 'fWGfGGWfWGfWW', 'true'),
(2, 1, '0', 'NULL', 1, NULL, 'if', 53, NULL, 8191, 'g6v', 'ttGxGjG');
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
(1, 1, 1, 2, '(voice) (uncredited)', 4095, 1),
(2, 1, 1, 2, '(voice)', NULL, 2);
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
(1, 2, 1, 2, '00');
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
(1, 1, 1, 'Bulgaria', 'COM1');
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
(1, 2, 1, '8.0', 'Dhu');
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
(1, 2, 1, '_ZCCFF', NULL);
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
(1, '22qS1v', '[us]', 2048, '222222222222222222222', 'yY', 'R');
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
(1, 'character-name-in-title', 'cRRccRR');
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
(1, 'Downey Jr., Robert', NULL, 8191, NULL, 'hh', 'jjrjr', '00', '1e100'),
(2, 'Downey Jr., Robert', '', 4, 'NIL', 'HTc', 'Vi', 'Infinity', '0Kt''t0I8t8It0K08t''''tIIKKt0');
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
(1, 'Other Character', '0', 1321, 'true', '9q99', 'NULL'),
(2, 'Voice Character', '777', 5044, ' 1Gxd', '44N66NKfN', 'UpUhUphhhphppU'),
(3, 'Other Character', NULL, 939, '', '_U_U_U_', 'Inf');
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
(1, '__proto__', '__dict__', 1, 2009, NULL, 'if', 1, 1059, NULL, '__dict__', 'y%Kbb%L%%HyHPHbHKb%y');
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
(1, 2, 'me%EmElGe', '', '-Infinity', 'kxxx%xx%%', 'true', 'uGbDDGIGGeGGIb%buDbubu%%bD%bI%G');
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
(1, 1, 'Inf', NULL, 2, 128, 'GW', 318, 15, 2124, '', NULL),
(2, 1, 'True', '', 2, 5107, NULL, 162, NULL, 255, 'NULL', '-Infinity'),
(3, 1, 'JQ', 'aaaEbE', 1, 31, '__proto__', 323, 547, 426, 'cccccc', 'z');
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
(1, 1, 1, 2, NULL, 1982, 1),
(2, 1, 1, 1, '(uncredited)', 242, 3),
(3, 2, 1, 1, '(voice) (uncredited)', 1271, 3);
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
(1, 1, 3, 2),
(2, 1, 3, 2);
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
(1, 1, 1, 3, 'KKPKBX'),
(2, 1, 1, 3, 'mn');
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
(1, 1, 2, 'Bulgaria', 'qTj'),
(2, 1, 1, 'Bulgaria', '7Zo'),
(3, 1, 1, 'Bulgaria', '88''88t8n''''');
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
(1, 1, 1, '4.0', 'KY'),
(2, 1, 3, '8.0', '11o'),
(3, 1, 1, '8.0', NULL);
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
(1, 2, 1, 'QQTQTTTTQ', '___');
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
(1, 'PPVP', NULL, 1, 'rKC', NULL, 'j'),
(2, 'pZpZpHpyZyZsZHypZsZ', '[de]', 32, ' 6E f6', 'g', 'm5mmPymgg5Pg55'),
(3, 'gggggg', NULL, 2047, 'l33qy', 'D1h508hdk50', '8c88aa818c');
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
(1, 'character-name-in-title', NULL);
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
(1, 'Other Person', '', NULL, 'se5RR', 'false', 'dddmYYdrdrYYr', 'kIcWWdW', 'b3g3gb  3 3b33gbgb g'),
(2, 'Downey Jr., Robert', 'OllLLL', 512, 'WGUs8syI8', 'ssss', 'yo', 'Nu', 'H'),
(3, 'Other Person', 'C%b', 271, 'Inf', '00', '', 'UVVddUV', 'WW');
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
(1, 'Other Character', 'R5kR5uWPuWuPPRWu55Pu', 2047, '__proto__', 'eeQQQQ', NULL),
(2, 'Other Character', 'lllkkkkklVV', 5, '4 94', ' gg ', NULL);
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
(1, 'ZZZ', '0ep%m%w0mmu%0wwL0L00''m''0e''', 2, 2008, 6709, '', 4, 412, NULL, '', NULL),
(2, '0', NULL, 2, NULL, 128, 'lU6RJ', 2, 2047, 4095, '', 'pJ1pCE1J');
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
(1, 1, 'BBGSDBnnDDnrSrBSzzn', '%mm%mKKym%%y%%ymy', NULL, 'w4hwwhwIZ48h4w4h84Zw44ZIwwI', '__proto__', NULL);
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
(1, 1, 'Infinity', 'GMQ', 1, 157, '''T''T', 256, 6, NULL, '', 'NULL'),
(2, 2, 'ehePPePeehPhhhPPP', '00', 1, 1011, '', 8192, 1023, 64, 'LPT1', 'I_');
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
(1, 2, 2, 1, '(uncredited)', 5665, 1),
(2, 1, 2, 2, '(voice)', 2, 2);
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
(1, 2, 2, 1, 'None');
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
(1, 1, 2, 'USA', 'uoaa88'),
(2, 1, 2, 'Bulgaria', 'QSQQ ');
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
(1, 2, 1, '4.0', 'ka66'),
(2, 1, 2, '4.0', '11oo1-1o--o-o-ooo1o-o1-11-');
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
(1, 2, 2, 'kookoDvvowkvwkoovo', 'eddZeede');
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
(1, 'none', '[us]', 8, '999999999999999999999999999999', 'gVVVHIR6', '4''4''''''4''444''');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'null'),
(2, 'hero-sequel', 'LPT1'),
(3, 'character-name-in-title', 'ZZZz');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie'),
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
(1, 'Downey Jr., Robert', 'R9', 7, 'WWLL', 'yRRyyyy', '91 bp9t9tWGpttGp', '888', 'G G');
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
(1, 'Other Character', 'ssb', 3, NULL, 'Infinity', 'y33y3yIy');
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
(1, '3', '''v''pp''vp''''vvpvpvv''vpppvv', 3, 2010, 256, 'etteeMettMMtM', 64, 2047, 511, 'NaN', ''),
(2, 'k ksk skkssk ', 'True', 2, 2005, 3, 'iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii', 2047, 4095, 8, 'U6U6SSU', 'ANzNzNrNDwAzA6Dz64wNr'),
(3, '  33 gHnd', NULL, 2, 2012, 64, '000', 2896, 31, 2047, 'GG', '8V2K');
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
(1, 1, '%', 'COM1', 'S', 'FC', 'HIN', 'H''H'),
(2, 1, 'then', NULL, '  6A666Ae66A', 'te', 'kkss', '88');
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
(1, 2, 'Hp', '', 2, 512, 'h-mnn--', NULL, NULL, NULL, 'TlTTlTDDD', 'eExex_xExEE');
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
(1, 1, 3, NULL, '(voice) (uncredited)', 4, 2);
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
(1, 2, 2, 2),
(2, 3, 2, 3),
(3, 1, 2, 3);
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
(1, 3, 1, 3, NULL);
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
(1, 1, 2, 'Bulgaria', ' U UV UU VUUUUhUh VVU'),
(2, 1, 1, 'Bulgaria', 'EJg'),
(3, 1, 2, 'Bulgaria', 'NaN');
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
(1, 2, 1, '4.0', 'lAl6A6 '),
(2, 1, 1, '8.0', 'if'),
(3, 1, 1, '8.0', 'NUL');
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
(1, 3, 2, 2),
(2, 2, 1, 1),
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
(1, 1, 2, 'NULL', 'c0'),
(2, 1, 1, '', NULL);
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
(1, 'True', '[us]', 835, '', '__proto__', '00'),
(2, '', '[us]', 279, NULL, 'NaN', 'sssUsUUUsUUUsUU');
CREATE TABLE company_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO company_type VALUES
(1, 'production companies'),
(2, 'production companies'),
(3, 'production companies');
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
(1, 'hero-sequel', 'v'),
(2, 'marvel-cinematic-universe', '_TT____TT_TT_TT_TT__T_T____T');
CREATE TABLE kind_type (
  id BIGINT NOT NULL,
  kind VARCHAR NOT NULL,
  PRIMARY KEY (id)
);
INSERT INTO kind_type VALUES
(1, 'movie'),
(2, 'movie'),
(3, 'episode');
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
(1, 'Downey Jr., Robert', 'True', 3095, 'nZ', 'llllllllal', 'COM1', 'None', 'False');
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
(1, 'Voice Character', 'Inf', 191, 'Or''5O5', '77P7PP', NULL),
(2, 'Voice Character', '00', 38, '', '', '1J'),
(3, 'Other Character', '0', 669, '', '', 's');
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
(1, 'INF', NULL, 1, 2007, 33, '-06PXX6X6X6i', 2630, 1023, 4, 'p66ppppp6p66pp6', 'BoFFwwBF');
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
(1, 1, '22-Q0-', 'BRBsRgHHBR', 'zv', '', 'G', 'aF66HH');
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
(1, 1, '33nf3fn%n333%n%%n', 'cttJtt', 3, 334, '', 112, 384, 966, '', 'true'),
(2, 1, 'gXX', '73', 2, 7863, 'k', 128, 315, 1056, '_y', 'bWCbAWkFbkAb5CkFWWb'),
(3, 1, '388', '3ebw3ob55oeCb9bbC93', 2, 601, 'K', 64, 73, 358, 'rrIII1', 'NULL');
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
(1, 1, 1, 2, '(uncredited)', 756, 2),
(2, 1, 1, 3, '(voice)', 8024, 1);
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
(2, 1, 2, 1);
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
(1, 1, 2, 2, ''),
(2, 1, 2, 1, '-Infinity'),
(3, 1, 1, 1, 'Inf');
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
(1, 1, 2, 'USA', ''),
(2, 1, 3, 'USA', 'MJtZM'),
(3, 1, 1, 'Bulgaria', ' f X3fR3fR X 33XwXfw3XRX');
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
(1, 1, 1, '8.0', '0dd_0_dd_'),
(2, 1, 2, '4.0', '2JNf222f'),
(3, 1, 2, '4.0', 'g');
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
(1, 1, 3, '', '__dict__');
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
(1, 'then', '[us]', 145, '66', 'I', 'ts'),
(2, 'RiwJEVJRwJEVwERVJJxxVBwEExwiiERx', '[de]', 2739, 'o', 'LPT1', 'LPT1'),
(3, 'x', '[de]', 319, '1e100', 'lllBlBlBBBlBB', 'NIL');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'undefined'),
(2, 'hero-sequel', 'Z%nnZnZeZyeZZn');
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
(1, 'Downey Jr., Robert', 'h8-h8', 957, '1e100', '_''_70''''7', '2h222h2hhhh2', '-Infinity', '1'),
(2, 'Other Person', NULL, 724, NULL, '999999999999999999999999999999', 'HH0H0rH0rr', '999999999999999999999999999999', '');
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
(1, 'Voice Character', NULL, NULL, 'LPT1', 'Ey_', 'eLR'),
(2, 'Other Character', 'LPT1', 8166, 'godd%', 'III9', 'PD9__0P_09D'),
(3, 'Other Character', 'false', 7, 'Inf', 'Ct5', 'rququruuuu');
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
(1, '1e100', 'then', 1, 2009, 2048, '', 8191, 598, NULL, 'h666hhh66', 'if'),
(2, 't', 'LPT1', 1, 2006, 128, 'NaN', 183, 491, 8, '00', '0'),
(3, 'then', 'zjXjj', 1, 2012, NULL, '', 1024, 63, 6, 'Scunthorpe', '0');
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
(1, 2, 'i', '__dict__', 'KK', 'HHqYqYd', 'None', '77T');
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
(1, 3, 'jjjj', 'JJHnH''''', 1, 64, 'D jfDN', 252, 65, 259, 'O%3%O%%', NULL),
(2, 2, 'Inf', 'Lq', 1, 1409, '%z', 8, 6, 480, '0', '00');
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
(1, 1, 3, 1, '(voice)', 327, 2),
(2, 2, 1, 2, '(voice)', NULL, 1);
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
(1, 1, 1, 2, 'null'),
(2, 3, 2, 1, '_LKSS'),
(3, 3, 3, 1, 'None');
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
(1, 2, 2, 'Bulgaria', NULL),
(2, 1, 2, 'USA', 'else');
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
(1, 3, 2, '8.0', '-');
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
(1, 3, 2, 2),
(2, 1, 2, 3),
(3, 3, 2, 2);
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
(1, 1, 2, '', ''),
(2, 2, 1, 'm', '''mll ''''T T''l ''lmm''lm''llml ''l');
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
(1, 'then', '[us]', 145, '66', 'I', 'ts'),
(2, 'RiwJEVJRwJEVwERVJJxxVBwEExwiiERx', '[de]', 2739, 'o', 'LPT1', 'LPT1'),
(3, 'x', '[de]', 319, '1e100', 'lllBlBlBBBlBB', 'NIL');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'undefined'),
(2, 'hero-sequel', 'Z%nnZnZeZyeZZn');
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
(1, 'Downey Jr., Robert', 'h8-h8', 957, '1e100', '_''_70''''7', '2h222h2hhhh2', '-Infinity', '1'),
(2, 'Other Person', NULL, 724, NULL, '999999999999999999999999999999', 'HH0H0rH0rr', '999999999999999999999999999999', '');
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
(1, 'Voice Character', NULL, NULL, 'LPT1', 'Ey_', 'eLR'),
(2, 'Other Character', 'LPT1', 8166, 'godd%', 'III9', 'PD9__0P_09D'),
(3, 'Other Character', 'false', 7, 'Inf', 'Ct5', 'rququruuuu');
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
(1, '1e100', 'then', 1, 2009, 2048, '', 8191, 598, NULL, 'h666hhh66', 'if'),
(2, 't', 'LPT1', 1, 2006, 128, 'NaN', 183, 491, 8, '00', '0'),
(3, 'then', 'zjXjj', 1, 2012, NULL, '', 1024, 63, NULL, NULL, 'Scunthorpe');
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
(1, 2, '0', NULL, NULL, NULL, '__dict__', 'KK');
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
(1, 2, 'HHqYqYd', 'None', 1, NULL, NULL, NULL, NULL, NULL, 'JJHnH''''', NULL),
(2, 2, '1', NULL, 1, 252, '1', NULL, 1, NULL, NULL, NULL);
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
(1, 2, 2, 2, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, 8, 2);
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
(1, 2, 2, 2, '00'),
(2, 1, 3, 2, NULL),
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
(1, 2, 2, 'USA', '1'),
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
(1, 3, 2, 1),
(2, 2, 2, 2),
(3, 3, 3, 1);
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
(1, 'then', '[us]', 145, '66', 'I', 'ts'),
(2, 'RiwJEVJRwJEVwERVJJxxVBwEExwiiERx', '[de]', 2739, 'o', 'LPT1', 'LPT1'),
(3, 'x', '[de]', 319, '1e100', 'lllBlBlBBBlBB', 'NIL');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'undefined'),
(2, 'hero-sequel', 'Z%nnZnZeZyeZZn');
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
(1, 'Downey Jr., Robert', 'h8-h8', 957, '1e100', '_''_70''''7', '2h222h2hhhh2', '-Infinity', '1'),
(2, 'Other Person', NULL, 724, NULL, '999999999999999999999999999999', 'HH0H0rH0rr', '999999999999999999999999999999', '');
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
(1, 'Voice Character', NULL, NULL, 'LPT1', 'Ey_', 'eLR'),
(2, 'Other Character', 'LPT1', NULL, NULL, 'godd%', 'III9'),
(3, 'Other Character', NULL, NULL, NULL, 'false', '1');
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
(1, '1', NULL, 1, NULL, NULL, 'rququruuuu', NULL, 1, NULL, NULL, NULL),
(2, '1', NULL, 1, NULL, 1, '1', NULL, 1, NULL, 'if', NULL),
(3, '1', NULL, 1, NULL, 2006, '1', NULL, NULL, 183, '1', NULL);
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
(1, 2, '00', '0', NULL, 'zjXjj', NULL, '1');
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
(1, 2, '', '1', 1, 6, 'Scunthorpe', 1, NULL, NULL, NULL, NULL),
(2, 2, '__dict__', 'KK', 1, NULL, NULL, 1, NULL, 1, NULL, NULL);
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
(1, 2, 2, 2, NULL, NULL, 2),
(2, 2, 2, NULL, NULL, 252, 2);
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
(1, 2, 2, 2, NULL),
(2, 2, 2, 2, 'Lq'),
(3, 1, 2, 2, '%z');
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
(1, 2, 2, '4.0', NULL);
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
(1, 2, 1, 2),
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
(1, 2, 1, '1', NULL),
(2, 2, 1, '1', NULL);
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
(1, 'then', '[us]', 145, '66', 'I', 'ts'),
(2, 'RiwJEVJRwJEVwERVJJxxVBwEExwiiERx', '[de]', 2739, 'o', 'LPT1', 'LPT1'),
(3, 'x', '[de]', 319, '1e100', 'lllBlBlBBBlBB', 'NIL');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'undefined'),
(2, 'hero-sequel', 'Z%nnZnZeZyeZZn');
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
(1, 'Downey Jr., Robert', 'h8-h8', 957, '1e100', '_''_70''''7', '2h222h2hhhh2', '-Infinity', '1'),
(2, 'Other Person', NULL, 724, NULL, '999999999999999999999999999999', 'HH0H0rH0rr', '999999999999999999999999999999', '');
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
(1, 'Voice Character', NULL, NULL, 'LPT1', 'Ey_', 'eLR'),
(2, 'Other Character', 'LPT1', 8166, 'godd%', 'III9', 'PD9__0P_09D'),
(3, 'Other Character', 'false', 7, 'Inf', 'Ct5', 'rququruuuu');
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
(1, '1e100', '', 1, NULL, 2009, '1', 1, NULL, NULL, '1', NULL),
(2, 'h666hhh66', 'if', 1, 2006, NULL, NULL, 2006, 128, 1, NULL, '1'),
(3, '1', '1', 1, NULL, 1, NULL, NULL, 1, NULL, NULL, '1');
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
(1, 2, '', '1', NULL, '1', NULL, NULL);
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
(1, 2, '0', NULL, 1, NULL, '__dict__', 1, NULL, 1, NULL, 'None'),
(2, 2, '77T', NULL, 1, NULL, 'JJHnH''''', NULL, 64, 1, NULL, '1');
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
(1, 2, 2, NULL, '(voice)', NULL, 2),
(2, 2, 2, NULL, '(voice)', NULL, 1);
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
(1, 2, 2, 2);
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
(1, 2, 2, 2, '1'),
(2, 2, 2, 2, NULL),
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
(1, 3, 2, 'USA', '1'),
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
(1, 2, 2, '4.0', NULL);
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
(1, 1, 1, 1),
(2, 2, 2, 2),
(3, 2, 3, 2);
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
(1, 1, 2, '_LKSS', NULL),
(2, 2, 1, '', NULL);
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
(1, 'then', '[us]', 145, '66', 'I', 'ts'),
(2, 'RiwJEVJRwJEVwERVJJxxVBwEExwiiERx', '[de]', 2739, 'o', 'LPT1', 'LPT1'),
(3, 'x', '[de]', 319, '1e100', 'lllBlBlBBBlBB', 'NIL');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'undefined'),
(2, 'hero-sequel', 'Z%nnZnZeZyeZZn');
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
(1, 'Downey Jr., Robert', 'h8-h8', 957, '1e100', '_''_70''''7', '2h222h2hhhh2', '-Infinity', '1'),
(2, 'Other Person', NULL, 724, NULL, '999999999999999999999999999999', 'HH0H0rH0rr', '999999999999999999999999999999', '');
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
(1, 'Voice Character', NULL, NULL, 'LPT1', 'Ey_', 'eLR'),
(2, 'Other Character', 'LPT1', 8166, 'godd%', 'III9', 'PD9__0P_09D'),
(3, 'Other Character', 'false', 7, 'Inf', 'Ct5', 'rququruuuu');
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
(1, '1e100', 'then', 1, 2009, 2048, '', 8191, 598, NULL, 'h666hhh66', 'if'),
(2, 't', 'LPT1', 1, 2006, 128, 'None', 183, 491, 8, '00', '0'),
(3, 'then', 'zjXjj', 1, 2012, NULL, '', 1024, 63, 6, 'Scunthorpe', '0');
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
(1, 2, 'i', '__dict__', 'KK', 'HHqYqYd', 'None', '77T');
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
(1, 3, 'jjjj', 'JJHnH''''', 1, 64, 'D jfDN', 252, 65, 259, 'O%3%O%%', NULL),
(2, 2, 'Inf', 'Lq', 1, 1409, '%z', 8, 6, 480, '0', '00');
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
(1, 1, 3, 1, '(voice)', 327, 2),
(2, 2, 1, 2, '(voice)', NULL, 1);
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
(1, 1, 1, 2, 'null'),
(2, 3, 2, 1, '_LKSS'),
(3, 3, 3, 1, 'None');
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
(1, 2, 2, 'Bulgaria', NULL),
(2, 1, 2, 'USA', 'else');
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
(1, 3, 2, '8.0', '-');
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
(1, 3, 2, 2),
(2, 1, 2, 3),
(3, 3, 2, 2);
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
(1, 1, 2, '', ''),
(2, 2, 1, 'm', '''mll ''''T T''l ''lmm''lm''llml ''l');
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
(1, '00', '[ru]', 8843, '', 'FALSE', 'then');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'PnUU09U9n99n9i09'),
(2, 'marvel-cinematic-universe', 'c22Q06nQ0z2J62nQcn'),
(3, 'hero-sequel', 'EEE');
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
(1, 'Downey Jr., Robert', '1e100', 279, 'pp18UjE', 'INF', '', 'false', NULL),
(2, 'Other Person', 'nsnsnnn', 128, 'None', NULL, 'NIL', '''', ''),
(3, 'Other Person', '', NULL, 'mmm', 'NUL', NULL, 'qW3b1h W', '');
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
(1, 'Other Character', '_', 8191, 'zlzR', 'True', 'VdodoVoEwdwEQdEo'),
(2, 'Other Character', 'dYB Art3', 0, '', 'Inf', 'FALSE'),
(3, 'Voice Character', '-R', 804, 'Rf''''B', 'gItg', 'S9ae-');
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
(1, 'oaSToTTvv', '0', 1, 2012, 377, 'true', 7, 1023, 125, NULL, 'LPT1'),
(2, '  ', '%j1%%', 1, 2010, 634, 'fhfff', 2329, NULL, 2805, '33N7x57P3PP3ooNo3373o', '%%Eo%EH%oHKo');
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
(1, 1, 'CCCy', 'U', '1ER1Q', 'ff-JhBBhh', 'qqqq', '1e100'),
(2, 1, 'COM1', 'zMMzo', 'TTTo', 'NIL', 'qqqdii', 'NaN'),
(3, 1, '', 'NIL', 'Jid7JxGG', 'qVqk', 'INF', 'HHrHTTHr');
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
(1, 1, 'else', 'False', 2, 587, NULL, 149, 314, 1, NULL, 'sjmPsZZ');
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
(1, 1, 1, 1, '(uncredited)', NULL, 1),
(2, 3, 1, 1, '(voice)', 7377, 2);
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
(1, 2, 3, 1),
(2, 2, 1, 3),
(3, 2, 3, 1);
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
(1, 1, 1, 1, 'hBBhhhBhBB'),
(2, 2, 1, 2, 'q');
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
(1, 2, 2, 'Bulgaria', 'idda9');
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
(1, 1, 1, '8.0', 'Scunthorpe'),
(2, 2, 1, '8.0', 't8tBwnwn');
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
(1, 1, 2, '', 'True'),
(2, 1, 1, '', '3--3'),
(3, 1, 2, 'V466067VV446060V7V04SVVS7SV64V60', 'COM1');
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
(1, '00', '[ru]', 8843, '', 'FALSE', 'then');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'PnUU09U9n99n9i09'),
(2, 'marvel-cinematic-universe', 'c22Q06nQ0z2J62nQcn'),
(3, 'hero-sequel', 'EEE');
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
(1, 'Downey Jr., Robert', '1e100', 279, 'pp18UjE', 'INF', '', 'false', NULL),
(2, 'Other Person', 'nsnsnnn', 128, 'None', NULL, 'NIL', '''', ''),
(3, 'Other Person', '', NULL, 'mmm', 'NUL', NULL, 'qW3b1h W', '');
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
(1, 'Other Character', '_', 8191, 'zlzR', 'True', 'VdodoVoEwdwEQdEo'),
(2, 'Other Character', 'dYB Art3', 0, '', 'Inf', 'FALSE'),
(3, 'Voice Character', '''', 804, 'Rf''''B', 'gItg', 'S9ae-');
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
(1, 'oaSToTTvv', '0', 1, 2012, 377, 'true', 7, 1023, 125, NULL, 'LPT1'),
(2, '  ', '%j1%%', 1, 2010, 634, 'fhfff', 2329, NULL, 2805, '33N7x57P3PP3ooNo3373o', '%%Eo%EH%oHKo');
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
(1, 1, 'CCCy', 'U', '1ER1Q', 'ff-JhBBhh', 'qqqq', '1e100'),
(2, 1, 'COM1', 'zMMzo', 'TTTo', 'NIL', 'qqqdii', 'NaN'),
(3, 1, '', 'NIL', 'Jid7JxGG', 'qVqk', 'INF', 'HHrHTTHr');
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
(1, 1, 'else', 'False', 2, 587, NULL, 149, 314, 1, NULL, 'sjmPsZZ');
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
(1, 1, 1, 1, '(uncredited)', NULL, 1),
(2, 3, 1, 1, '(voice)', 7377, 2);
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
(1, 2, 3, 1),
(2, 2, 1, 3),
(3, 2, 3, 1);
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
(1, 1, 1, 1, 'hBBhhhBhBB'),
(2, 2, 1, 2, 'q');
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
(1, 2, 2, 'Bulgaria', 'idda9');
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
(1, 1, 1, '8.0', 'Scunthorpe'),
(2, 2, 1, '8.0', 't8tBwnwn');
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
(1, 1, 2, '', 'True'),
(2, 1, 1, '', '3--3'),
(3, 1, 2, 'V466067VV446060V7V04SVVS7SV64V60', 'COM1');
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
(1, '00', '[ru]', 8843, '', 'FALSE', 'then');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'PnUU09U9n99n9i09'),
(2, 'marvel-cinematic-universe', 'c22Q06nQ0z2J62nQcn'),
(3, 'hero-sequel', 'EEE');
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
(1, 'Downey Jr., Robert', '1e100', 279, 'pp18UjE', 'INF', '', 'false', NULL),
(2, 'Other Person', 'nsnsnnn', 128, 'None', NULL, 'NIL', '''', ''),
(3, 'Other Person', '', NULL, 'mmm', 'NUL', NULL, 'qW3b1h W', '');
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
(1, 'Other Character', '_', 8191, 'zlzR', 'True', 'VdodoVoEwdwEQdEo'),
(2, 'Other Character', 'dYB Art3', 0, '', 'Inf', 'FALSE'),
(3, 'Voice Character', '-R', 804, 'Rf''''B', 'gItg', 'S9ae-');
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
(1, 'oaSToTTvv', '0', 1, 2012, 377, 'true', 7, 1023, 125, NULL, 'LPT1'),
(2, '  ', '%j1%%', 1, 2010, 634, 'fhfff', 2329, NULL, 2805, '33N7x57P3PP3ooNo3373o', '%%Eo%EH%oHKo');
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
(1, 1, 'CCCy', 'U', '1ER1Q', 'ff-JhBBhh', 'qqqq', '1e100'),
(2, 1, 'COM1', 'zMMzo', 'TTTo', 'NIL', 'qqqdii', 'NaN'),
(3, 1, '', 'NIL', 'Jid7JxGG', 'qVqk', 'INF', 'HHrHTTHr');
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
(1, 1, 'else', 'False', 2, 587, NULL, 149, 314, 1, NULL, 'sjmPsZZ');
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
(1, 1, 1, 1, '(uncredited)', NULL, 1),
(2, 3, 1, 1, '(voice)', 7377, 2);
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
(1, 2, 3, 1),
(2, 2, 1, 3),
(3, 2, 3, 1);
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
(1, 1, 1, 1, 'hBBhhhBhBB'),
(2, 2, 1, 2, 'q');
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
(1, 2, 2, 'Bulgaria', 'idda9');
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
(1, 1, 1, '8.0', 'Scunthorpe'),
(2, 2, 1, '8.0', 't8tBwnwn');
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
(1, 1, 2, '', 'True'),
(2, 1, 1, '', '3--3'),
(3, 1, 2, 'V466067VV446060V7V04SVVS7SV64V60', 'COM1');
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
(1, '', '[us]', 4924, 'wtSSt', 'FFFFj', 'pvvtvGvVt');
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
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'False'),
(2, 'marvel-cinematic-universe', 'NIL'),
(3, 'character-name-in-title', 'True');
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
(1, 'Downey Jr., Robert', 'ssssL', 1096, '', '', '999999999999999999999999999999', '', 'true'),
(2, 'Downey Jr., Robert', 'WkZZVWWWWWVZ', 1427, '-Infinity', 'fKifKjUiKKvvfijKKUjiiijUjUvKciK', 'yyyyy', '55555k5', '1RRRm11V1V'),
(3, 'Other Person', '6YYzEYzY', 30, NULL, 'None', 'COM1', NULL, 'Y-Y8YY8-YY-YY8YYY8YY');
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
(1, 'Voice Character', 'QQQQ', 327, 'pF0pF0mFp0', 'false', '__dict__'),
(2, 'Other Character', 'none', 4299, 'NaN', 'Rz0', '');
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
(1, 'ek', 'if', 1, 2009, 325, 'Scunthorpe', NULL, NULL, 9, NULL, NULL),
(2, 'O15-7p', 'RPVR', 2, NULL, 1384, 'LPT1', 1554, 4053, NULL, '', 'WEmm');
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
(1, 1, '1jj1', 'XXXXXXXXXXXXXX', 'ununaaaaaannnn', 'b', 'B007B7yB70', 'p8p');
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
(1, 1, '00', 'J', 1, 95, 'L', 726, 970, 122, '', 'X'),
(2, 2, 'glgsV', 'f4qfuqfu4qfq4quu4fq', 1, NULL, 'ElEll', 639, 1675, 4743, NULL, ''),
(3, 1, 'NNNNNNNNNNNNN', 'TRUE', 2, 8209, 'z', 128, 92, 43, 'JB', NULL);
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
(1, 2, 1, 1, '(uncredited)', 297, 1);
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
(2, 1, 1, 1),
(3, 2, 1, 1);
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
(1, 2, 1, 1, 'XUUUUUI'),
(2, 2, 1, 3, 'True');
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
(1, 1, 1, 'Bulgaria', '-Infinity');
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
(1, 1, 2, '4.0', 'g'),
(2, 2, 1, '8.0', 'NONMfMO');
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
(1, 3, 1, '00', 'C');
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
(1, '', '[us]', 4924, 'wtSSt', 'RPVR', 'pvvtvGvVt');
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
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'False'),
(2, 'marvel-cinematic-universe', 'NIL'),
(3, 'character-name-in-title', 'True');
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
(1, 'Downey Jr., Robert', 'ssssL', 1096, '', '', '999999999999999999999999999999', '', 'true'),
(2, 'Downey Jr., Robert', 'WkZZVWWWWWVZ', 1427, '-Infinity', 'fKifKjUiKKvvfijKKUjiiijUjUvKciK', 'yyyyy', '55555k5', '1RRRm11V1V'),
(3, 'Other Person', '6YYzEYzY', 30, NULL, 'None', 'COM1', NULL, 'Y-Y8YY8-YY-YY8YYY8YY');
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
(1, 'Voice Character', 'QQQQ', 327, 'pF0pF0mFp0', 'false', '__dict__'),
(2, 'Other Character', 'none', 4299, 'NaN', 'Rz0', '');
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
(1, 'ek', 'if', 1, 2009, 325, 'Scunthorpe', NULL, NULL, 9, NULL, NULL),
(2, 'O15-7p', 'RPVR', 2, NULL, 1384, 'LPT1', 1554, 4053, NULL, '', 'WEmm');
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
(1, 1, '1jj1', 'XXXXXXXXXXXXXX', 'ununaaaaaannnn', 'b', 'B007B7yB70', 'p8p');
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
(1, 1, '00', 'J', 1, 95, 'L', 726, 970, 122, '', 'X'),
(2, 2, 'glgsV', 'f4qfuqfu4qfq4quu4fq', 1, NULL, 'ElEll', 639, 1675, 4743, NULL, ''),
(3, 1, 'NNNNNNNNNNNNN', 'TRUE', 2, 8209, 'z', 128, 92, 43, 'JB', NULL);
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
(1, 2, 1, 1, '(uncredited)', 297, 1);
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
(2, 1, 1, 1),
(3, 2, 1, 1);
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
(1, 2, 1, 1, 'XUUUUUI'),
(2, 2, 1, 3, 'True');
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
(1, 1, 1, 'Bulgaria', '-Infinity');
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
(1, 1, 2, '4.0', 'g'),
(2, 2, 1, '8.0', 'NONMfMO');
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
(1, 3, 1, '00', 'C');
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
(1, '', '[us]', 4924, 'wtSSt', 'FFFFj', 'pvvtvGvVt');
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
(1, 'rating'),
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'False'),
(2, 'marvel-cinematic-universe', 'NIL'),
(3, 'character-name-in-title', 'True');
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
(1, 'Downey Jr., Robert', 'ssssL', 1096, '', '', '999999999999999999999999999999', '', 'true'),
(2, 'Downey Jr., Robert', 'WkZZVWWWWWVZ', 1427, '-Infinity', 'fKifKjUiKKvvfijKKUjiiijUjUvKciK', 'yyyyy', '55555k5', '1RRRm11V1V'),
(3, 'Other Person', '6YYzEYzY', 30, NULL, 'None', 'COM1', NULL, 'Y-Y8YY8-YY-YY8YYY8YY');
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
(1, 'Voice Character', 'QQQQ', 327, 'pF0pF0mFp0', 'false', '__dict__'),
(2, 'Other Character', 'none', 4299, 'NaN', 'Rz0', '');
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
(1, 'ek', 'if', 1, 2009, 325, 'Scunthorpe', NULL, NULL, 9, NULL, NULL),
(2, 'O15-7p', 'RPVR', 2, NULL, 1384, 'LPT1', 1554, 4053, NULL, '', 'WEmm');
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
(1, 1, '1jj1', 'XXXXXXXXXXXXXX', 'ununaaaaaannnn', 'b', 'B007B7yB70', 'p8p');
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
(1, 1, '00', 'J', 1, 95, 'L', 726, 970, 122, '', 'X'),
(2, 2, 'glgsV', 'f4qfuqfu4qfq4quu4fq', 1, NULL, 'ElEll', 639, 1675, 4743, NULL, ''),
(3, 1, 'NNNNNNNNNNNNN', 'TRUE', 2, 8209, 'z', 128, 92, 43, 'JB', NULL);
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
(1, 2, 1, 1, '(uncredited)', 297, 1);
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
(2, 1, 1, 1),
(3, 2, 1, 1);
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
(1, 2, 1, 1, 'XUUUUUI'),
(2, 2, 1, 3, 'if');
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
(1, 1, 1, 'Bulgaria', '-Infinity');
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
(1, 1, 2, '4.0', 'g'),
(2, 2, 1, '8.0', 'NONMfMO');
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
(1, 3, 1, '00', 'C');
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
(1, '7B', '[ru]', 1023, 'NIL', 'TRUE', 'i4');
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'Infinity'),
(2, 'hero-sequel', 'LxxxLxYxLLLYxLLYxYx'),
(3, 'marvel-cinematic-universe', 'ONOONONOONONN');
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
(1, 'Downey Jr., Robert', 'aa', 4575, 'if', 'qM', 'Infinity', 'g--n', 'E'),
(2, 'Other Person', '-Infinity', 4095, 'DuuDuuuuDuDDuuDDDDD', 'IaI', 'A0  Ayw0wy50w0y0050A0005yw0wyA 5', '', 'eKX1e');
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
(1, 'Voice Character', NULL, 74, 'DDDDVI', '', 'iiiaaiiaiiiaaiaaiiiaiaa');
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
(1, 'FI', 'nluuuhR', 1, 2006, 414, '50', 8, 209, 32, '', 'if'),
(2, '0', 'pGr8G', 1, NULL, 928, 'hhhh1hh1', 63, 64, 8, '', NULL);
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
(1, 1, '', 'true', '', '999999999999999999999999999999', '', 'OORe'),
(2, 2, '0', '', 'if', 'RR', 'eeeeeeeeeeee', '__proto__');
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
(1, 2, 'XXBXXB88Tn8', NULL, 1, 4, '999999999999999999999999999999', NULL, 3454, 15, 'bb00', 'qq'),
(2, 2, 'then', 'E ET', 1, 4095, 'J', 31, 256, 686, 'Infinity', 'False');
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
(1, 1, 2, 1, NULL, 15, 1),
(2, 1, 2, 1, '(voice)', 64, 1);
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
(2, 1, 2, 1),
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
(1, 1, 1, 2, NULL),
(2, 1, 1, 1, '0');
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
(1, 2, 3, 'Bulgaria', '00');
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
(1, 2, 1, '4.0', 'False');
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
(2, 2, 2, 2);
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
(1, 1, 1, 'www', '');
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
(1, '7B', '[ru]', 1023, 'NIL', 'TRUE', 'i4');
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'Infinity'),
(2, 'hero-sequel', 'LxxxLxYxLLLYxLLYxYx'),
(3, 'marvel-cinematic-universe', 'ONOONONOONONN');
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
(1, 'Downey Jr., Robert', 'pGr8G', 4575, 'if', 'qM', 'Infinity', 'g--n', 'E'),
(2, 'Other Person', '-Infinity', 4095, 'DuuDuuuuDuDDuuDDDDD', 'IaI', 'A0  Ayw0wy50w0y0050A0005yw0wyA 5', '', 'eKX1e');
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
(1, 'Voice Character', NULL, 74, 'DDDDVI', '', 'iiiaaiiaiiiaaiaaiiiaiaa');
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
(1, 'FI', 'nluuuhR', 1, 2006, 414, '50', 8, 209, 32, '', 'if'),
(2, '0', 'pGr8G', 1, NULL, 928, 'hhhh1hh1', 63, 64, 8, '', NULL);
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
(1, 1, '', 'true', '', '999999999999999999999999999999', '', 'OORe'),
(2, 2, '0', '', 'if', 'RR', 'eeeeeeeeeeee', '__proto__');
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
(1, 2, 'XXBXXB88Tn8', NULL, 1, 4, '999999999999999999999999999999', NULL, 3454, 15, 'bb00', 'qq'),
(2, 2, 'then', 'E ET', 1, 4095, 'J', 31, 256, 686, 'Infinity', 'False');
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
(1, 1, 2, 1, NULL, 15, 1),
(2, 1, 2, 1, '(voice)', 64, 1);
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
(2, 1, 2, 1),
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
(1, 1, 1, 2, NULL),
(2, 1, 1, 1, '0');
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
(1, 2, 3, 'Bulgaria', '00');
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
(1, 2, 1, '4.0', 'False');
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
(2, 2, 2, 2);
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
(1, 1, 1, 'www', '');
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
(1, '7B', '[ru]', 1023, 'NIL', 'TRUE', 'i4');
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'Infinity'),
(2, 'hero-sequel', 'LxxxLxYxLLLYxLLYxYx'),
(3, 'marvel-cinematic-universe', 'ONOONONOONONN');
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
(1, 'Downey Jr., Robert', 'aa', 4575, 'if', 'qM', 'Infinity', 'g--n', 'E'),
(2, 'Other Person', '-Infinity', 4095, 'DuuDuuuuDuDDuuDDDDD', 'IaI', 'A0  Ayw0wy50w0y0050A0005yw0wyA 5', '', 'eKX1e');
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
(1, 'Voice Character', NULL, 74, 'DDDDVI', '', 'iiiaaiiaiiiaaiaaiiiaiaa');
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
(1, 'FI', 'nluuuhR', 1, 2006, 414, '50', 8, 209, 32, '', 'if'),
(2, '0', 'pGr8G', 1, NULL, 928, 'hhhh1hh1', 63, 64, 8, '', NULL);
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
(1, 1, '', 'true', '', '999999999999999999999999999999', '', 'OORe'),
(2, 2, '0', '', 'if', 'RR', 'eeeeeeeeeeee', '__proto__');
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
(1, 2, 'XXBXXB88Tn8', NULL, 1, 4, '-Infinity', NULL, 3454, 15, 'bb00', 'qq'),
(2, 2, 'then', 'E ET', 1, 4095, 'J', 31, 256, 686, 'Infinity', 'False');
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
(1, 1, 2, 1, NULL, 15, 1),
(2, 1, 2, 1, '(voice)', 64, 1);
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
(2, 1, 2, 1),
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
(1, 1, 1, 2, NULL),
(2, 1, 1, 1, '0');
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
(1, 2, 3, 'Bulgaria', '00');
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
(1, 2, 1, '4.0', 'False');
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
(2, 2, 2, 2);
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
(1, 1, 1, 'www', '');
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
(1, 'true', '[de]', 357, NULL, 'FfcScfFc', 'True'),
(2, '__dict__', '[ru]', 626, 'LPT1', 'NaN', 'yyBG''vv');
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'i'),
(2, 'hero-sequel', '857D');
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
(1, 'Other Person', 'nil', 3819, 'ddddd', 'DV', 'cjc', 'psJqb', 'HjHHI'),
(2, 'Downey Jr., Robert', 'Inf', 497, '__dict__', 'RP''', '5', 'm', NULL);
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
(1, 'Other Character', NULL, 4592, 'NNNNNNNNNNNNNNN', '-Infinity', 'jejDDej');
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
(1, 'jjOw1WJOj', 'LLLL', 1, 2010, 83, '0', 2883, 2754, NULL, 'NUL', 'FALSE');
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
(1, 2, 'Scunthorpe', 'CCCC', '6g', 'L5L155', 'none', ''),
(2, 1, '7HN3VNV', 'M', 'C', 'qh', 'NUL', '88K');
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
(1, 1, 'cPP5Pkckk', 'true', 3, 4387, 'NULL', 424, NULL, 408, '', NULL),
(2, 1, 'VV''''', '', 2, 6579, '__dict__', 1024, NULL, 4953, '%%tt%t%%', 'NULL');
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
(1, 2, 1, 1, '(voice)', 80, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 987, 1);
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
(2, 1, 2, 1),
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
(1, 1, 2, 1, 'NaN'),
(2, 1, 2, 2, ''),
(3, 1, 2, 2, 'n');
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
(1, 1, 1, 'Bulgaria', 'o9gHogHO'),
(2, 1, 1, 'USA', '9');
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
(1, 1, 3, '8.0', ''''),
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
(1, 2, 2, '1e100', 'tt');
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
(1, 'true', '[de]', 357, NULL, 'FfcScfFc', 'True'),
(2, '__dict__', '[ru]', 626, 'LPT1', 'NaN', 'yyBG''vv');
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'i'),
(2, 'hero-sequel', '857D');
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
(1, 'Other Person', 'nil', 3819, 'ddddd', 'DV', 'cjc', 'psJqb', 'HjHHI'),
(2, 'Downey Jr., Robert', 'Inf', 497, '__dict__', 'RP''', '5', 'm', NULL);
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
(1, 'Other Character', NULL, 4592, 'NNNNNNNNNNNNNNN', '-Infinity', 'jejDDej');
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
(1, 'jjOw1WJOj', 'LLLL', 1, 2010, 83, '0', 2883, 2754, NULL, 'NUL', 'FALSE');
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
(1, 2, 'Scunthorpe', 'CCCC', '6g', 'L5L155', 'none', ''),
(2, 1, '7HN3VNV', 'M', 'C', 'qh', 'NUL', '88K');
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
(1, 1, 'cPP5Pkckk', 'true', 3, 4387, 'NULL', 424, NULL, 408, '', NULL),
(2, 1, 'VV''''', '', 2, 6579, '__dict__', 1024, NULL, 4953, '%%tt%t%%', 'NULL');
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
(1, 2, 1, 1, '(voice)', 80, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 987, 1);
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
(1, 1, 2, 1, 'NaN'),
(2, 1, 2, 2, ''),
(3, 1, 2, 2, 'n');
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
(1, 1, 1, 'Bulgaria', 'o9gHogHO'),
(2, 1, 1, 'USA', '9');
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
(1, 1, 3, '8.0', ''''),
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
(1, 2, 2, '1e100', 'tt');
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
(1, 'true', '[de]', 357, NULL, 'FfcScfFc', 'True'),
(2, '__dict__', '[ru]', 626, 'LPT1', 'NaN', 'yyBG''vv');
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
(2, 'countries'),
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'i'),
(2, 'hero-sequel', '857D');
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
(1, 'Other Person', 'nil', 3819, 'ddddd', 'DV', 'cjc', 'psJqb', 'HjHHI'),
(2, 'Downey Jr., Robert', 'Inf', 497, '__dict__', 'RP''', '5', 'm', NULL);
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
(1, 'Other Character', NULL, 4592, 'NNNNNNNNNNNNNNN', '-Infinity', 'jejDDej');
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
(1, 'jjOw1WJOj', 'LLLL', 1, 2010, 83, '0', 2883, 2754, NULL, 'NUL', 'FALSE');
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
(1, 2, 'Scunthorpe', 'CCCC', '6g', 'L5L155', 'none', ''),
(2, 1, '7HN3VNV', 'M', 'C', 'qh', 'NUL', '88K');
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
(1, 1, 'cPP5Pkckk', 'true', 3, 4387, 'NULL', 424, NULL, 408, '', NULL),
(2, 1, 'VV''''', '', 2, 6579, '__dict__', 1024, NULL, 4953, '%%tt%t%%', 'NULL');
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
(1, 2, 1, 1, '(voice)', 80, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 987, 1);
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
(2, 1, 2, 1),
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
(1, 1, 2, 1, 'NaN'),
(2, 1, 2, 2, ''),
(3, 1, 2, 2, 'n');
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
(1, 1, 1, 'Bulgaria', 'o9gHogHO'),
(2, 1, 1, 'USA', '');
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
(2, 1, 2, '4.0', NULL);
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
-- Generated database 027/100
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
(1, 'true', '[de]', 357, NULL, 'FfcScfFc', 'True'),
(2, '__dict__', '[ru]', 626, 'LPT1', 'NaN', 'yyBG''vv');
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
(2, 'countries'),
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'i'),
(2, 'hero-sequel', '857D');
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
(1, 'Other Person', 'nil', 3819, 'ddddd', 'DV', 'cjc', 'psJqb', 'HjHHI'),
(2, 'Downey Jr., Robert', 'Inf', 497, '__dict__', 'RP''', '5', 'm', NULL);
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
(1, 'Other Character', NULL, 4592, 'NNNNNNNNNNNNNNN', '-Infinity', 'jejDDej');
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
(1, 'jjOw1WJOj', 'LLLL', 1, 2010, 83, '0', 2883, 2754, NULL, 'NUL', 'FALSE');
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
(1, 2, 'Scunthorpe', 'CCCC', '6g', 'L5L155', 'none', ''),
(2, 1, '7HN3VNV', 'M', 'C', 'qh', 'NUL', '88K');
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
(1, 1, 'cPP5Pkckk', 'true', 3, 4387, 'NULL', 424, NULL, 408, '', NULL),
(2, 1, 'VV''''', '', 2, 6579, '__dict__', 1024, NULL, 4953, '%%tt%t%%', 'NULL');
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
(1, 2, 1, 1, '(voice)', 80, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 987, 1);
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
(2, 1, 2, 1),
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
(1, 1, 2, 1, 'NaN'),
(2, 1, 2, 2, ''),
(3, 1, 2, 2, 'n');
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
(1, 1, 1, 'Bulgaria', 'o9gHogHO'),
(2, 1, 1, 'USA', '9');
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
(1, 1, 3, '8.0', ''''),
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
(1, 2, 2, '1e100', 'tt');
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
(1, 'then', '[us]', NULL, 'FxFF33x33_F3Vx_', NULL, 'none');
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
(2, 'countries');
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
(1, 'episode'),
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
(1, 'Other Person', '999999999999999999999999999999', 31, 'None', 'ttt-', 'd', '55jmB351', NULL),
(2, 'Downey Jr., Robert', '_nDnjz', 343, '', '', 'Scunthorpe', '', '');
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
(1, 'Other Character', '', 8713, 'rB6r', 'INF', '');
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
(1, 'else', 'NUL', 1, 2006, 290, NULL, 966, NULL, 9999, '', 'gzzda9'),
(2, '7-mK99K9K_f', 'True', 1, 2011, 80, 'False', 10000, 9999, NULL, '111G11', 'vOv'),
(3, 'undefined', '0', 2, 2007, 1, 'f', NULL, 1092, 4108, 'e', 'HtttHpllplHpttlHHll');
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
(1, 1, 'I', 'none', '9F', 'yyUBcZc', 'P5QQPP55Q5', NULL);
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
(1, 1, 's', 'ttiitt', 2, 9999, '6y', NULL, 672, 110, '''F', 'aMMaaMMaaaMMbaM'),
(2, 3, '', 'fffff', 1, 1144, 'if', 121, 8192, 6, 'Yi7', 'n'),
(3, 1, 'x7xVVuVL7l', '', 1, 2170, 'bDb', 3976, 9999, 1, 'wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww', '');
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
(1, 1, 3, 1, '(voice)', 7724, 3),
(2, 2, 2, NULL, '(uncredited)', 31, 1),
(3, 1, 2, 1, '(voice)', 10000, 2);
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
(1, 2, 2, 1);
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
(1, 2, 1, 1, '9'),
(2, 3, 1, 1, '''pgp%Gh%m'),
(3, 2, 1, 1, '');
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
(1, 2, 1, 'USA', '90H0'),
(2, 1, 2, 'Bulgaria', 'LNL');
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
(1, 3, 1, '4.0', 'gdddgdddd');
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
(1, 2, 3, 1);
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
(1, 2, 2, 'FkVFVkVkVFV', 'ss'),
(2, 2, 2, '__dict__', 'G-'),
(3, 1, 2, 'LPT1', 'INF');
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
(1, 'then', '[us]', NULL, 'FxFF33x33_F3Vx_', NULL, 'none');
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
(2, 'countries');
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
(1, 'episode'),
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
(1, 'Other Person', '999999999999999999999999999999', 31, 'None', 'ttt-', 'none', '55jmB351', NULL),
(2, 'Downey Jr., Robert', '_nDnjz', 343, '', '', 'Scunthorpe', '', '');
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
(1, 'Other Character', '', 8713, 'rB6r', 'INF', '');
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
(1, 'else', 'NUL', 1, 2006, 290, NULL, 966, NULL, 9999, '', 'gzzda9'),
(2, '7-mK99K9K_f', 'True', 1, 2011, 80, 'False', 10000, 9999, NULL, '111G11', 'vOv'),
(3, 'undefined', '0', 2, 2007, 1, 'f', NULL, 1092, 4108, 'e', 'HtttHpllplHpttlHHll');
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
(1, 1, 'I', 'none', '9F', 'yyUBcZc', 'P5QQPP55Q5', NULL);
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
(1, 1, 's', 'ttiitt', 2, 9999, '6y', NULL, 672, 110, '''F', 'aMMaaMMaaaMMbaM'),
(2, 3, '', 'fffff', 1, 1144, 'if', 121, 8192, 6, 'Yi7', 'n'),
(3, 1, 'x7xVVuVL7l', '', 1, 2170, 'bDb', 3976, 9999, 1, 'wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww', '');
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
(1, 1, 3, 1, '(voice)', 7724, 3),
(2, 2, 2, NULL, '(uncredited)', 31, 1),
(3, 1, 2, 1, '(voice)', 10000, 2);
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
(1, 2, 2, 1);
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
(1, 2, 1, 1, '9'),
(2, 3, 1, 1, '''pgp%Gh%m'),
(3, 2, 1, 1, '');
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
(1, 2, 1, 'USA', '90H0'),
(2, 1, 2, 'Bulgaria', 'LNL');
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
(1, 3, 1, '4.0', 'gdddgdddd');
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
(1, 2, 3, 1);
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
(1, 2, 2, 'FkVFVkVkVFV', 'ss'),
(2, 2, 2, '__dict__', 'G-'),
(3, 1, 2, 'LPT1', 'INF');
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
(1, 'then', '[us]', NULL, 'FxFF33x33_F3Vx_', NULL, 'none');
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
(2, 'countries');
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
(1, 'episode'),
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
(1, 'Other Person', '999999999999999999999999999999', 31, 'None', 'ttt-', 'd', '55jmB351', NULL),
(2, 'Downey Jr., Robert', '_nDnjz', 343, '', '', 'Scunthorpe', '', '');
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
(1, 'Other Character', '', 8713, 'rB6r', 'INF', '');
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
(1, 'else', 'NUL', 1, 2006, 290, NULL, 966, NULL, 9999, '', 'gzzda9'),
(2, '7-mK99K9K_f', 'True', 1, 2011, 80, 'False', 10000, 9999, NULL, '111G11', 'vOv'),
(3, 'undefined', '0', 2, 2007, 1, 'f', NULL, 1092, 4108, 'e', 'HtttHpllplHpttlHHll');
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
(1, 1, 'I', 'none', '9F', 'yyUBcZc', 'P5QQPP55Q5', NULL);
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
(1, 1, 's', 'ttiitt', 2, 9999, '6y', NULL, 672, 110, '''F', 'aMMaaMMaaaMMbaM'),
(2, 3, '', 'fffff', 1, 1144, 'if', 121, 8192, 6, 'Yi7', 'n'),
(3, 1, 'x7xVVuVL7l', '', 1, 2170, 'bDb', 3976, 9999, 1, 'wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww', '');
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
(1, 1, 3, 1, '(voice)', 7724, 3),
(2, 2, 2, NULL, '(uncredited)', 31, 1),
(3, 1, 2, 1, '(voice)', 10000, 2);
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
(1, 2, 2, 1);
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
(1, 2, 1, 1, '9'),
(2, 3, 1, 1, '''pgp%Gh%m'),
(3, 2, 1, 1, '');
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
(1, 2, 1, 'USA', '90H0'),
(2, 1, 2, 'Bulgaria', 'LNL');
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
(1, 3, 1, '4.0', 'gdddgdddd');
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
(1, 2, 3, 1),
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
(1, 2, 2, 'ss', NULL),
(2, 2, 2, '1', NULL),
(3, 2, 1, '1', NULL);
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
(1, 'then', '[us]', NULL, 'FxFF33x33_F3Vx_', NULL, 'none');
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
(2, 'countries');
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
(1, 'episode'),
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
(1, 'Other Person', '999999999999999999999999999999', 31, 'None', 'ttt-', 'd', '55jmB351', NULL),
(2, 'Downey Jr., Robert', '_nDnjz', 343, '', '', 'Scunthorpe', '', '');
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
(1, 'Other Character', '', 8713, 'rB6r', 'INF', '');
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
(1, 'else', 'NUL', 1, 2006, 290, NULL, 966, NULL, 9999, '', 'gzzda9'),
(2, '7-mK99K9K_f', 'True', 1, 2011, 80, 'False', 10000, 9999, NULL, '111G11', 'vOv'),
(3, 'undefined', '0', 2, 2007, 1, 'f', NULL, 1092, 4108, 'e', 'HtttHpllplHpttlHHll');
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
(1, 1, 'I', 'none', '9F', 'yyUBcZc', 'P5QQPP55Q5', NULL);
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
(1, 1, 's', 'ttiitt', 2, 9999, '6y', NULL, 672, 110, '''F', 'aMMaaMMaaaMMbaM'),
(2, 3, '', 'fffff', 1, 1144, 'if', 121, 8192, 6, 'Yi7', 'n'),
(3, 1, 'x7xVVuVL7l', '', 1, 2170, 'bDb', 3976, 9999, 1, 'wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww', '');
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
(1, 1, 3, 1, '(voice)', 7724, 3),
(2, 2, 2, NULL, '(uncredited)', 31, 1),
(3, 1, 2, 1, '(voice)', 10000, 2);
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
(1, 2, 2, 1);
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
(1, 2, 1, 1, '9'),
(2, 3, 1, 1, '''pgp%Gh%m'),
(3, 2, 1, 1, '');
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
(1, 2, 1, 'USA', '90H0'),
(2, 1, 2, 'Bulgaria', 'LNL');
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
(1, 3, 1, '4.0', 'gdddgdddd');
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
(1, 2, 3, 1);
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
(1, 2, 2, 'FkVFVkVkVFV', 'ss'),
(2, 2, 2, '__dict__', 'G-'),
(3, 1, 2, 'LPT1', 'HtttHpllplHpttlHHll');
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
(1, 'DttDDtDtDDtD', '[de]', 1, 'Inf', 'wUwwwwUUUUww', ''),
(2, '9B''Rs11''BBY9sG1''Y', '[ru]', 255, 'Ujj88', 'A9t', '0'),
(3, 'AtAsaAaAAAstta', '[de]', 2342, '', 'Qnp00nnpn', 'l');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'ooOJ92');
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
(1, 'Other Person', 'then', NULL, NULL, 'NaN', 'w', 'DtttttDDDDttttttDtttt', 'NIL');
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
(1, 'Voice Character', NULL, 4, 'Kr_', 'null', '');
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
(1, 'mm', '2WW2W2', 2, 2011, NULL, 'biC', 511, 318, 5, 'm', 'True'),
(2, 'null', 'lllllLllLlLLllLL', 2, 2008, NULL, '', 8192, 1, 128, '0', NULL);
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
(1, 1, 'eee', 'FwL', NULL, 'LYOYOvE', '', '5'),
(2, 1, '7agEN7N IsgssNza7z', 'u', NULL, NULL, 'MM', 'False');
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
(1, 2, '', 'True', 1, NULL, 'pHIoIJIpOpwJIHHOMJ8THwJwp', NULL, 1024, NULL, 'Wm', 'null'),
(2, 2, '5', 'y_', 1, 1023, NULL, NULL, 403, 108, 'nFx', NULL);
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
(1, 1, 1, 1, '(voice)', 667, 3);
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
(1, 2, 2, 1),
(2, 2, 3, 1),
(3, 2, 2, 1);
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
(1, 1, 1, 3, 'Ou0O0u');
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
(1, 1, 1, 'Bulgaria', 'tda'),
(2, 1, 1, 'Bulgaria', 'wwwwwwLLLLwLwwL');
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
(2, 2, 1, '8.0', 'e1ead'),
(3, 2, 1, '4.0', '_');
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
(1, 1, 2, 1),
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
(1, 1, 1, '0', 'HlllDHvDSSLLLLD9SoSL9'),
(2, 1, 1, '0x', 'NULL');
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
(1, 'DttDDtDtDDtD', '[de]', 1, 'Inf', 'wUwwwwUUUUww', ''),
(2, '9B''Rs11''BBY9sG1''Y', '[ru]', 255, 'Ujj88', 'A9t', '0'),
(3, 'AtAsaAaAAAstta', '[de]', 2342, '', 'Qnp00nnpn', 'l');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'ooOJ92');
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
(1, 'Other Person', '', NULL, NULL, NULL, 'NaN', 'w', 'DtttttDDDDttttttDtttt');
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
(1, 'Voice Character', NULL, NULL, NULL, NULL, '1');
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
(1, '1', NULL, 2, NULL, NULL, '', NULL, 1, NULL, NULL, '1'),
(2, '1', NULL, 2, 2006, 318, '1', NULL, NULL, 1, NULL, NULL);
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
(1, 1, 'lllllLllLlLLllLL', NULL, '1', '', '1', NULL),
(2, 1, '1', NULL, NULL, NULL, NULL, NULL);
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
(1, 2, '', NULL, 2, NULL, 'LYOYOvE', NULL, 1, 1, NULL, NULL),
(2, 2, '1', NULL, 2, NULL, NULL, 1, NULL, 1, NULL, NULL);
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
(1, 1, 2, NULL, NULL, NULL, 2);
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
(1, 2, 2, 2),
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
(1, 2, 1, 'USA', NULL),
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
(1, 2, 1, '4.0', '1'),
(2, 2, 1, '4.0', NULL),
(3, 1, 1, '4.0', '1');
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
(1, 2, 2, 2),
(2, 1, 2, 2);
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
(2, 1, 1, '1', NULL);
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
(1, 'DttDDtDtDDtD', '[de]', 1, 'Inf', 'wUwwwwUUUUww', ''),
(2, '9B''Rs11''BBY9sG1''Y', '[ru]', 255, 'Ujj88', 'A9t', '0'),
(3, 'AtAsaAaAAAstta', '[de]', 2342, '', 'Qnp00nnpn', 'l');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'ooOJ92');
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
(1, 'Other Person', 'then', NULL, NULL, 'null', 'w', 'DtttttDDDDttttttDtttt', 'NIL');
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
(1, 'Voice Character', NULL, 4, 'Kr_', 'null', '');
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
(1, 'mm', '2WW2W2', 2, 2011, NULL, 'biC', 511, 318, 5, 'm', 'True'),
(2, 'null', 'lllllLllLlLLllLL', 2, 2008, NULL, '', 8192, 1, 128, '0', NULL);
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
(1, 1, 'eee', 'FwL', NULL, 'LYOYOvE', '', '5'),
(2, 1, '7agEN7N IsgssNza7z', 'u', NULL, NULL, 'MM', 'False');
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
(1, 2, '', 'True', 1, NULL, 'pHIoIJIpOpwJIHHOMJ8THwJwp', NULL, 1024, NULL, 'Wm', 'null'),
(2, 2, '5', 'y_', 1, 1023, NULL, NULL, 403, 108, 'nFx', NULL);
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
(1, 1, 1, 1, '(voice)', 667, 3);
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
(1, 2, 2, 1),
(2, 2, 3, 1),
(3, 2, 2, 1);
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
(1, 1, 1, 3, 'Ou0O0u');
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
(1, 1, 1, 'Bulgaria', 'tda'),
(2, 1, 1, 'Bulgaria', 'wwwwwwLLLLwLwwL');
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
(2, 2, 1, '8.0', 'e1ead'),
(3, 2, 1, '4.0', '_');
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
(1, 1, 2, 1),
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
(1, 1, 1, '0', 'HlllDHvDSSLLLLD9SoSL9'),
(2, 1, 1, '0x', 'NULL');
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
(1, 'DttDDtDtDDtD', '[de]', 1, 'Inf', 'wUwwwwUUUUww', ''),
(2, '9B''Rs11''BBY9sG1''Y', '[ru]', 255, 'Ujj88', 'A9t', '0'),
(3, 'AtAsaAaAAAstta', '[de]', 2342, '', 'Qnp00nnpn', 'l');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'ooOJ92');
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
(1, 'Other Person', 'then', NULL, NULL, 'NaN', 'w', 'DtttttDDDDttttttDtttt', 'NIL');
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
(1, 'Voice Character', NULL, 4, 'Kr_', 'null', '');
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
(1, 'mm', '2WW2W2', 2, 2011, NULL, 'biC', 511, 318, 5, 'm', 'True'),
(2, 'null', 'lllllLllLlLLllLL', 2, 2008, NULL, '', 8192, 1, 128, '0', NULL);
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
(1, 1, 'eee', 'FwL', NULL, 'LYOYOvE', '', '5'),
(2, 1, '7agEN7N IsgssNza7z', 'u', NULL, NULL, 'MM', 'False');
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
(1, 2, '', 'True', 1, NULL, 'pHIoIJIpOpwJIHHOMJ8THwJwp', NULL, 1024, NULL, 'Wm', 'null'),
(2, 2, '5', 'y_', 1, 1023, NULL, NULL, 403, 108, 'nFx', NULL);
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
(1, 1, 1, 1, '(voice)', 667, 3);
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
(1, 2, 2, 1),
(2, 2, 3, 1),
(3, 2, 2, 1);
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
(1, 1, 1, 3, 'Ou0O0u');
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
(1, 1, 1, 'Bulgaria', 'tda'),
(2, 1, 1, 'Bulgaria', 'wwwwwwLLLLwLwwL');
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
(2, 2, 1, '8.0', 'e1ead'),
(3, 2, 1, '4.0', 'wwwwwwLLLLwLwwL');
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
(1, 1, 2, 1),
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
(1, 1, 1, '0', 'HlllDHvDSSLLLLD9SoSL9'),
(2, 1, 1, '0x', 'NULL');
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
(1, 'DttDDtDtDDtD', '[de]', 1, 'Inf', 'wUwwwwUUUUww', ''),
(2, '9B''Rs11''BBY9sG1''Y', '[ru]', 255, 'Ujj88', 'A9t', '0'),
(3, 'AtAsaAaAAAstta', '[de]', 2342, '', 'Qnp00nnpn', 'l');
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
(1, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'ooOJ92');
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
(1, 'Other Person', 'then', NULL, NULL, 'NaN', 'w', 'DtttttDDDDttttttDtttt', 'NIL');
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
(1, 'Voice Character', NULL, 4, 'Kr_', 'null', '');
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
(1, 'mm', '2WW2W2', 2, 2011, NULL, 'biC', 511, 318, 5, 'm', 'True'),
(2, 'null', 'lllllLllLlLLllLL', 2, 2008, NULL, '', 8192, 0, 128, '0', NULL);
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
(1, 1, 'eee', 'FwL', NULL, 'LYOYOvE', '', '5'),
(2, 1, '7agEN7N IsgssNza7z', 'u', NULL, NULL, 'MM', 'False');
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
(1, 2, '', 'True', 1, NULL, 'pHIoIJIpOpwJIHHOMJ8THwJwp', NULL, 1024, NULL, 'Wm', 'null'),
(2, 2, '5', 'y_', 1, 1023, NULL, NULL, 403, 108, 'nFx', NULL);
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
(1, 1, 1, 1, '(voice)', 667, 3);
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
(1, 2, 2, 1),
(2, 2, 3, 1),
(3, 2, 2, 1);
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
(1, 1, 1, 3, 'Ou0O0u');
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
(1, 1, 1, 'Bulgaria', 'tda'),
(2, 1, 1, 'Bulgaria', 'wwwwwwLLLLwLwwL');
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
(2, 2, 1, '8.0', 'e1ead'),
(3, 2, 1, '4.0', '_');
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
(1, 1, 2, 1),
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
(1, 1, 1, '0', 'HlllDHvDSSLLLLD9SoSL9'),
(2, 1, 1, '0x', 'NULL');
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
(1, 'if', '[us]', 4095, '''wTw0mT''iFwp99Fi9p09w0', NULL, 'w'),
(2, 'wBBGGrrBGpsspBsrsw GG', '[ru]', NULL, 'kkkkkkkkkkkkkk', 'QRQ', 'nil');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', ''),
(2, 'character-name-in-title', '');
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
(1, 'Other Person', 'G0D0DE', 2, '', 'ieiwei', '00', 'IKvNkDD-N-NDIID', '11111111111111'),
(2, 'Downey Jr., Robert', '', 2047, 'rggfrrWrffWarRWfWf', 'FFFFFFFFFFF', 'MDDA', 'TRUE', 'TRUE'),
(3, 'Downey Jr., Robert', NULL, 4095, NULL, '', '__proto__', '_nRn_R6MM6', NULL);
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
(1, 'Other Character', 'e', 128, 'else', 'xL23GLLLG2GG2G2L23LLG2LGxLmGL3Gm', 'AO');
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
(1, 'm', 'NaN', 1, 2006, 512, 'L', 256, 2275, NULL, 'ATu  gA-V', 'zzzz'),
(2, 'Scunthorpe', NULL, 1, NULL, 4267, '', NULL, NULL, 802, 'Scunthorpe', 'COM1');
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
(1, 2, 'ii', 'then', NULL, 'XX', '5Kf', 'ELLRL'),
(2, 3, 'Inf', 'dQBxc', 'Eqq', 'IMOvv', 'I55', 'B'),
(3, 1, '333', 'GAG3G-Axx', '5', '', '''17', 'VVVVVVVVVVVVVVVVV');
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
(1, 1, 'vBJjJvv1BJ17B', 'lUlU', 2, 271, 'n8Zn W8', 2, 255, 5, 'LPT1', 'D');
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
(1, 1, 2, NULL, '(uncredited)', 437, 1);
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
(1, 2, 1, 2),
(2, 2, 1, 2),
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
(1, 1, 1, 2, 'WWWWWWWWW');
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
(1, 2, 1, 'Bulgaria', 'WxCUF 2o WL2xxWLxWCUL'),
(2, 1, 1, 'USA', 'ZCFF''qZUFWpUZI''q''ZpU'),
(3, 1, 1, 'USA', 'nil');
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
(1, 1, 1, '4.0', 'true'),
(2, 1, 1, '8.0', 'TRUE');
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
(1, 2, 2, 2),
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
(1, 1, 2, 'z', NULL),
(2, 3, 2, 'hwh3swhshhs3b', 'ruruNro5N5o');
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
(1, 'if', '[us]', 4095, '''wTw0mT''iFwp99Fi9p09w0', NULL, 'w'),
(2, 'wBBGGrrBGpsspBsrsw GG', '[ru]', NULL, 'kkkkkkkkkkkkkk', 'QRQ', 'nil');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', ''),
(2, 'character-name-in-title', '');
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
(1, 'Other Person', 'G0D0DE', 2, '', 'ieiwei', '00', 'IKvNkDD-N-NDIID', '11111111111111'),
(2, 'Downey Jr., Robert', '', 2047, 'rggfrrWrffWarRWfWf', 'FFFFFFFFFFF', 'MDDA', 'TRUE', 'TRUE'),
(3, 'Downey Jr., Robert', NULL, 4095, NULL, '', '__proto__', '_nRn_R6MM6', NULL);
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
(1, 'Other Character', 'e', 128, 'else', 'xL23GLLLG2GG2G2L23LLG2LGxLmGL3Gm', 'AO');
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
(1, 'm', 'NaN', 1, 2006, 512, 'L', 256, 2275, NULL, 'ATu  gA-V', 'zzzz'),
(2, 'GAG3G-Axx', NULL, 1, NULL, 4267, '', NULL, NULL, 802, 'Scunthorpe', 'COM1');
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
(1, 2, 'ii', 'then', NULL, 'XX', '5Kf', 'ELLRL'),
(2, 3, 'Inf', 'dQBxc', 'Eqq', 'IMOvv', 'I55', 'B'),
(3, 1, '333', 'GAG3G-Axx', '5', '', '''17', 'VVVVVVVVVVVVVVVVV');
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
(1, 1, 'vBJjJvv1BJ17B', 'lUlU', 2, 271, 'n8Zn W8', 2, 255, 5, 'LPT1', 'D');
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
(1, 1, 2, NULL, '(uncredited)', 437, 1);
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
(1, 2, 1, 2),
(2, 2, 1, 2),
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
(1, 1, 1, 2, 'WWWWWWWWW');
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
(1, 2, 1, 'Bulgaria', 'WxCUF 2o WL2xxWLxWCUL'),
(2, 1, 1, 'USA', 'ZCFF''qZUFWpUZI''q''ZpU'),
(3, 1, 1, 'USA', 'nil');
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
(1, 1, 1, '4.0', 'true'),
(2, 1, 1, '8.0', 'TRUE');
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
(1, 2, 2, 2),
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
(1, 1, 2, 'z', NULL),
(2, 3, 2, 'hwh3swhshhs3b', 'ruruNro5N5o');
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
(1, 'if', '[us]', 4095, '''wTw0mT''iFwp99Fi9p09w0', NULL, 'w'),
(2, 'wBBGGrrBGpsspBsrsw GG', '[ru]', NULL, 'kkkkkkkkkkkkkk', 'QRQ', 'nil');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', ''),
(2, 'character-name-in-title', '');
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
(1, 'Other Person', 'G0D0DE', 2, '', 'ieiwei', '00', 'IKvNkDD-N-NDIID', '11111111111111'),
(2, 'Downey Jr., Robert', '', 2047, 'rggfrrWrffWarRWfWf', 'FFFFFFFFFFF', 'MDDA', 'TRUE', 'TRUE'),
(3, 'Downey Jr., Robert', NULL, 4095, NULL, '', '__proto__', '_nRn_R6MM6', NULL);
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
(1, 'Other Character', 'e', 128, 'else', 'xL23GLLLG2GG2G2L23LLG2LGxLmGL3Gm', 'AO');
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
(1, 'm', 'NaN', 1, 2006, 512, 'L', 256, 2275, NULL, 'ATu  gA-V', 'zzzz'),
(2, 'Scunthorpe', NULL, 1, NULL, 4267, '', NULL, NULL, 802, 'Scunthorpe', 'COM1');
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
(1, 2, 'ii', 'then', NULL, 'XX', '5Kf', 'ELLRL'),
(2, 3, 'Inf', 'dQBxc', 'Eqq', 'IMOvv', 'I55', 'B'),
(3, 1, 'LPT1', 'GAG3G-Axx', '5', '', '''17', 'VVVVVVVVVVVVVVVVV');
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
(1, 1, 'vBJjJvv1BJ17B', 'lUlU', 2, 271, 'n8Zn W8', 2, 255, 5, 'LPT1', 'D');
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
(1, 1, 2, NULL, '(uncredited)', 437, 1);
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
(1, 2, 1, 2),
(2, 2, 1, 2),
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
(1, 1, 1, 2, 'WWWWWWWWW');
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
(1, 2, 1, 'Bulgaria', 'WxCUF 2o WL2xxWLxWCUL'),
(2, 1, 1, 'USA', 'ZCFF''qZUFWpUZI''q''ZpU'),
(3, 1, 1, 'USA', 'nil');
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
(1, 1, 1, '4.0', 'true'),
(2, 1, 1, '8.0', 'TRUE');
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
(1, 2, 2, 2),
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
(1, 1, 2, 'z', NULL),
(2, 3, 2, 'hwh3swhshhs3b', 'ruruNro5N5o');
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
(1, 'if', '[us]', 4095, '''wTw0mT''iFwp99Fi9p09w0', NULL, 'w'),
(2, 'wBBGGrrBGpsspBsrsw GG', '[ru]', NULL, 'kkkkkkkkkkkkkk', 'QRQ', 'nil');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', ''),
(2, 'character-name-in-title', '');
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
(1, 'Other Person', 'G0D0DE', 2, '', 'ieiwei', '00', 'IKvNkDD-N-NDIID', '11111111111111'),
(2, 'Downey Jr., Robert', '', 2047, 'rggfrrWrffWarRWfWf', 'FFFFFFFFFFF', 'MDDA', 'TRUE', 'TRUE'),
(3, 'Downey Jr., Robert', NULL, 4095, NULL, '', '__proto__', '_nRn_R6MM6', NULL);
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
(1, 'Other Character', 'e', 128, 'else', 'xL23GLLLG2GG2G2L23LLG2LGxLmGL3Gm', 'AO');
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
(1, 'm', 'NaN', 1, 2006, 512, 'L', 256, 2275, NULL, 'ATu  gA-V', 'zzzz'),
(2, 'Scunthorpe', '1', 2, NULL, 1, NULL, NULL, 802, 1, NULL, 'COM1');
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
(1, 2, 'ii', 'then', NULL, 'XX', '5Kf', 'ELLRL'),
(2, 3, 'Inf', 'dQBxc', 'Eqq', 'IMOvv', 'I55', 'B'),
(3, 1, '333', 'GAG3G-Axx', '5', '', '''17', 'VVVVVVVVVVVVVVVVV');
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
(1, 1, 'vBJjJvv1BJ17B', 'lUlU', 2, 271, 'n8Zn W8', 2, 255, 5, 'LPT1', 'D');
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
(1, 1, 2, NULL, '(uncredited)', 437, 1);
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
(1, 2, 1, 2),
(2, 2, 1, 2),
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
(1, 1, 1, 2, 'WWWWWWWWW');
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
(1, 2, 1, 'Bulgaria', 'WxCUF 2o WL2xxWLxWCUL'),
(2, 1, 1, 'USA', 'ZCFF''qZUFWpUZI''q''ZpU'),
(3, 1, 1, 'USA', 'nil');
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
(1, 1, 1, '4.0', 'true'),
(2, 1, 1, '8.0', 'TRUE');
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
(1, 2, 2, 2),
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
(1, 1, 2, 'z', NULL),
(2, 3, 2, 'hwh3swhshhs3b', 'ruruNro5N5o');
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
(1, '2s2Hsss7s_27s', '[us]', 16, 'P', '77FFRFRR28', 'd');
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
(1, 'character-name-in-title', '0'),
(2, 'character-name-in-title', 'N');
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
(1, 'Downey Jr., Robert', 'u', 2337, 'then', '', 'm', '-Infinity', '_s_');
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
(1, 'Voice Character', '00', NULL, NULL, 'NaN', NULL);
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
(1, 'TRUE', 'vvvv', 1, 2009, 4, 'zwwzwwww', 105, 2, 2, '%', 'rouevHu');
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
(1, 1, '__dict__', 'ULwL8LLUUnn', 'lhOllO', '5', 'xssxkVskkxkxVVsksVx', '0'),
(2, 1, 'uu', 'NIL', NULL, 'if', '1e100', ''),
(3, 1, '', '99999--9---9', 'fd', '1e100', '__dict__', 'efiHHlHHlmeHmi');
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
(1, 1, 'lglyl', '', 1, 345, 'LLLLLLLL', 8192, NULL, 2048, NULL, NULL),
(2, 1, 'IIII', '63h63pT363yyyyyym6', 1, 31, NULL, 871, 3, 31, '''', '0'),
(3, 1, 'dhb''_VLhhb_V', '_Sc', 1, 688, NULL, 4, 127, 396, 'QQUQOB6OO', '%Xvqq2');
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
(1, 1, 1, 1, '(uncredited)', 5542, 1),
(2, 1, 1, 1, '(uncredited)', 398, 1),
(3, 1, 1, 1, '(voice)', 204, 1);
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
(1, 1, 1, 1, 'LL-LLL-dLLdL-Ldd--'),
(2, 1, 1, 1, '1e100'),
(3, 1, 1, 1, 'Infinity');
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
(1, 1, 1, 'USA', 'LPT1'),
(2, 1, 3, 'USA', 'RRRVVVVVRRRVR'),
(3, 1, 3, 'Bulgaria', 'pILUpyIqUIp');
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
(1, 1, 1, '8.0', '6mRRRm6Rm');
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
(1, 1, 1, 'ddX', 'L'),
(2, 1, 1, 'KE', ''),
(3, 1, 1, 'uuuuuuuuuuuuu', '9');
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
(1, '2s2Hsss7s_27s', '[us]', 16, 'P', '77FFRFRR28', 'd');
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
(1, 'character-name-in-title', '0'),
(2, 'character-name-in-title', 'N');
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
(1, 'Downey Jr., Robert', 'u', 2337, 'then', '', 'm', '-Infinity', '_s_');
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
(1, 'Voice Character', '00', NULL, NULL, 'NaN', NULL);
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
(1, 'TRUE', 'vvvv', 1, 2009, 4, 'zwwzwwww', 105, 2, 2, '%', 'rouevHu');
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
(1, 1, '__dict__', 'ULwL8LLUUnn', 'lhOllO', '5', 'xssxkVskkxkxVVsksVx', '0'),
(2, 1, 'uu', '77FFRFRR28', NULL, 'if', '1e100', ''),
(3, 1, '', '99999--9---9', 'fd', '1e100', '__dict__', 'efiHHlHHlmeHmi');
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
(1, 1, 'lglyl', '', 1, 345, 'LLLLLLLL', 8192, NULL, 2048, NULL, NULL),
(2, 1, 'IIII', '63h63pT363yyyyyym6', 1, 31, NULL, 871, 3, 31, '''', '0'),
(3, 1, 'dhb''_VLhhb_V', '_Sc', 1, 688, NULL, 4, 127, 396, 'QQUQOB6OO', '%Xvqq2');
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
(1, 1, 1, 1, '(uncredited)', 5542, 1),
(2, 1, 1, 1, '(uncredited)', 398, 1),
(3, 1, 1, 1, '(voice)', 204, 1);
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
(1, 1, 1, 1, 'LL-LLL-dLLdL-Ldd--'),
(2, 1, 1, 1, '1e100'),
(3, 1, 1, 1, 'Infinity');
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
(1, 1, 1, 'USA', 'LPT1'),
(2, 1, 3, 'USA', 'RRRVVVVVRRRVR'),
(3, 1, 3, 'Bulgaria', 'pILUpyIqUIp');
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
(1, 1, 1, '8.0', '6mRRRm6Rm');
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
(1, 1, 1, 'ddX', 'L'),
(2, 1, 1, 'KE', ''),
(3, 1, 1, 'uuuuuuuuuuuuu', '9');
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
(1, '2s2Hsss7s_27s', '[us]', 16, 'P', '77FFRFRR28', 'd');
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
(1, 'character-name-in-title', '0'),
(2, 'character-name-in-title', 'N');
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
(1, 'Downey Jr., Robert', 'u', 2337, 'then', '', 'm', '-Infinity', '_s_');
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
(1, 'Voice Character', '00', NULL, NULL, 'NaN', NULL);
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
(1, 'TRUE', 'vvvv', 1, 2009, 4, 'zwwzwwww', 105, 2, 2, '%', 'rouevHu');
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
(1, 1, '__dict__', 'ULwL8LLUUnn', 'lhOllO', 'TRUE', 'xssxkVskkxkxVVsksVx', '0'),
(2, 1, 'uu', 'NIL', NULL, 'if', '1e100', ''),
(3, 1, '', '99999--9---9', 'fd', '1e100', '__dict__', 'efiHHlHHlmeHmi');
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
(1, 1, 'lglyl', '', 1, 345, 'LLLLLLLL', 8192, NULL, 2048, NULL, NULL),
(2, 1, 'IIII', '63h63pT363yyyyyym6', 1, 31, NULL, 871, 3, 31, '''', '0'),
(3, 1, 'dhb''_VLhhb_V', '_Sc', 1, 688, NULL, 4, 127, 396, 'QQUQOB6OO', '%Xvqq2');
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
(1, 1, 1, 1, '(uncredited)', 5542, 1),
(2, 1, 1, 1, '(uncredited)', 398, 1),
(3, 1, 1, 1, '(voice)', 204, 1);
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
(1, 1, 1, 1, 'LL-LLL-dLLdL-Ldd--'),
(2, 1, 1, 1, '1e100'),
(3, 1, 1, 1, 'Infinity');
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
(1, 1, 1, 'USA', 'LPT1'),
(2, 1, 3, 'USA', 'RRRVVVVVRRRVR'),
(3, 1, 3, 'Bulgaria', 'pILUpyIqUIp');
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
(1, 1, 1, '8.0', '6mRRRm6Rm');
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
(1, 1, 1, 'ddX', 'L'),
(2, 1, 1, 'KE', ''),
(3, 1, 1, 'uuuuuuuuuuuuu', '9');
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
(1, '2s2Hsss7s_27s', '[us]', 16, 'P', '77FFRFRR28', 'd');
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
(1, 'character-name-in-title', '0'),
(2, 'character-name-in-title', 'N');
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
(1, 'Downey Jr., Robert', 'u', 2337, 'then', '', 'm', '-Infinity', '_s_');
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
(1, 'Voice Character', '00', NULL, NULL, 'NaN', NULL);
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
(1, 'TRUE', 'vvvv', 1, 2009, 4, '99999--9---9', 105, 2, 2, '%', 'rouevHu');
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
(1, 1, '__dict__', 'ULwL8LLUUnn', 'lhOllO', '5', 'xssxkVskkxkxVVsksVx', '0'),
(2, 1, 'uu', 'NIL', NULL, 'if', '1e100', ''),
(3, 1, '', '99999--9---9', 'fd', '1e100', '__dict__', 'efiHHlHHlmeHmi');
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
(1, 1, 'lglyl', '', 1, 345, 'LLLLLLLL', 8192, NULL, 2048, NULL, NULL),
(2, 1, 'IIII', '63h63pT363yyyyyym6', 1, 31, NULL, 871, 3, 31, '''', '0'),
(3, 1, 'dhb''_VLhhb_V', '_Sc', 1, 688, NULL, 4, 127, 396, 'QQUQOB6OO', '%Xvqq2');
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
(1, 1, 1, 1, '(uncredited)', 5542, 1),
(2, 1, 1, 1, '(uncredited)', 398, 1),
(3, 1, 1, 1, '(voice)', 204, 1);
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
(1, 1, 1, 1, 'LL-LLL-dLLdL-Ldd--'),
(2, 1, 1, 1, '1e100'),
(3, 1, 1, 1, 'Infinity');
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
(1, 1, 1, 'USA', 'LPT1'),
(2, 1, 3, 'USA', 'RRRVVVVVRRRVR'),
(3, 1, 3, 'Bulgaria', 'pILUpyIqUIp');
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
(1, 1, 1, '8.0', '6mRRRm6Rm');
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
(1, 1, 1, 'ddX', 'L'),
(2, 1, 1, 'KE', ''),
(3, 1, 1, 'uuuuuuuuuuuuu', '9');
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
(1, '2s2Hsss7s_27s', '[us]', 16, 'P', '77FFRFRR28', 'd');
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
(1, 'character-name-in-title', '0'),
(2, 'character-name-in-title', 'RRRVVVVVRRRVR');
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
(1, 'Downey Jr., Robert', 'u', 2337, 'then', '', 'm', '-Infinity', '_s_');
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
(1, 'Voice Character', '00', NULL, NULL, 'NaN', NULL);
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
(1, 'TRUE', 'vvvv', 1, 2009, 4, 'zwwzwwww', 105, 2, 2, '%', 'rouevHu');
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
(1, 1, '__dict__', 'ULwL8LLUUnn', 'lhOllO', '5', 'xssxkVskkxkxVVsksVx', '0'),
(2, 1, 'uu', 'NIL', NULL, 'if', '1e100', ''),
(3, 1, '', '99999--9---9', 'fd', '1e100', '__dict__', 'efiHHlHHlmeHmi');
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
(1, 1, 'lglyl', '', 1, 345, 'LLLLLLLL', 8192, NULL, 2048, NULL, NULL),
(2, 1, 'IIII', '63h63pT363yyyyyym6', 1, 31, NULL, 871, 3, 31, '''', '0'),
(3, 1, 'dhb''_VLhhb_V', '_Sc', 1, 688, NULL, 4, 127, 396, 'QQUQOB6OO', '%Xvqq2');
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
(1, 1, 1, 1, '(uncredited)', 5542, 1),
(2, 1, 1, 1, '(uncredited)', 398, 1),
(3, 1, 1, 1, '(voice)', 204, 1);
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
(1, 1, 1, 1, 'LL-LLL-dLLdL-Ldd--'),
(2, 1, 1, 1, '1e100'),
(3, 1, 1, 1, 'Infinity');
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
(1, 1, 1, 'USA', 'LPT1'),
(2, 1, 3, 'USA', 'RRRVVVVVRRRVR'),
(3, 1, 3, 'Bulgaria', 'pILUpyIqUIp');
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
(1, 1, 1, '8.0', '6mRRRm6Rm');
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
(1, 1, 1, 'ddX', 'L'),
(2, 1, 1, 'KE', ''),
(3, 1, 1, 'uuuuuuuuuuuuu', '9');
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
(1, 'iPiIGGPPPt', '[us]', 1452, '', 'false', 'FLHB9HLBB'),
(2, 'VB', '[us]', 135, 'm1UJT', 'DRm', ''),
(3, '-', '[us]', 210, '', 'm', NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'else'),
(3, 'hero-sequel', 'Jb');
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
(1, 'Other Person', 'M''33', 3835, NULL, NULL, 'o-o', NULL, '0');
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
(1, 'Other Character', 'H1YWWHYxYWYpHYW', 1130, ' i Z333 Z', 'true', NULL),
(2, 'Other Character', 'E', 0, 'tG', '44EE', NULL),
(3, 'Voice Character', '7', 588, '_ ', 'Q%y', '0blN0');
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
(1, 'iuPJv', '__cOlIlOooF-IIl_c--u', 3, 2005, 4215, '3', NULL, 3783, NULL, 'false', 'LPT1'),
(2, 'z''8', 'else', 3, 2008, 245, 'h35r', 354, 2345, 256, 'uuud', 'Scunthorpe'),
(3, 'MhQ', '88IWW8WaNI', 2, 2011, 4637, 'HyP9PHPEly', 437, 7800, 280, '', 'RDRG');
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
(1, 1, '0', 'ojxxjmoxjxmmoxx', 'INF', 'xWWEXWX', '__dict__', 'PE');
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
(1, 3, 'NaN', 'CSCCSS', 3, 2852, 'aVVLy4O', 2601, 2047, 626, 'Y', NULL),
(2, 2, 'KKKhKKhKhKKK', '6Xnp', 1, 863, 'kLLkkkLLkkkLLkkkLLLLkLL', 74, 1942, 7861, '66666666', 'NllXNllXl--'),
(3, 3, '_W__WW00WW0_W_WW_W_W0', 'COM1', 3, NULL, '66dd66zzdz6d', 1476, 386, 1974, NULL, 'UUYU');
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
(1, 1, 2, 3, '(uncredited)', 3933, 1);
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
(1, 3, 1, 1);
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
(1, 3, 1, 1, 'NfkppkpfNokkk'),
(2, 3, 1, 1, 'dpdQtQw'),
(3, 1, 3, 2, 'NaN');
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
(1, 1, 1, '4.0', 'YF');
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
(2, 3, 3),
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
(1, 2, 1, 3),
(2, 2, 3, 3),
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
(1, 1, 1, '', 'ssNNNNN%'),
(2, 1, 1, '__dict__', '');
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
(1, 'iPiIGGPPPt', '[us]', 1452, '', 'false', 'FLHB9HLBB'),
(2, 'VB', '[us]', 135, 'm1UJT', 'DRm', ''),
(3, '-', '[us]', 210, '', 'm', NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'else'),
(3, 'hero-sequel', 'Jb');
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
(1, 'Other Person', 'M''33', 3835, NULL, NULL, 'o-o', NULL, '0');
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
(1, 'Other Character', 'H1YWWHYxYWYpHYW', 1130, ' i Z333 Z', 'true', NULL),
(2, 'Other Character', 'E', 0, 'tG', '44EE', NULL),
(3, 'Voice Character', '7', 588, '_ ', 'Q%y', '0blN0');
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
(1, 'iuPJv', '__cOlIlOooF-IIl_c--u', 3, 2005, 4215, '3', NULL, 3783, NULL, 'false', 'LPT1'),
(2, 'z''8', 'else', 3, 2008, 245, 'h35r', 354, 2345, 256, 'uuud', 'Scunthorpe'),
(3, 'MhQ', '88IWW8WaNI', 2, 2011, 4637, 'HyP9PHPEly', 437, 7800, 280, '', 'RDRG');
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
(1, 1, '0', 'ojxxjmoxjxmmoxx', 'INF', 'xWWEXWX', '__dict__', 'PE');
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
(1, 3, 'NaN', 'CSCCSS', 3, 2852, 'aVVLy4O', 2601, 2047, 626, 'Y', NULL),
(2, 2, 'KKKhKKhKhKKK', '6Xnp', 1, 863, 'kLLkkkLLkkkLLkkkLLLLkLL', 74, 1942, 7861, '66666666', 'NllXNllXl--'),
(3, 3, '_W__WW00WW0_W_WW_W_W0', 'COM1', 3, NULL, '66dd66zzdz6d', 1476, 386, 1974, NULL, 'UUYU');
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
(1, 1, 2, 3, '(uncredited)', 3933, 1);
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
(1, 3, 1, 1);
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
(1, 3, 1, 1, 'NfkppkpfNokkk'),
(2, 3, 2, 1, 'dpdQtQw'),
(3, 1, 3, 2, 'NaN');
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
(1, 1, 1, '4.0', 'YF');
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
(2, 3, 3),
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
(1, 2, 1, 3),
(2, 2, 3, 3),
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
(1, 1, 1, '', 'ssNNNNN%'),
(2, 1, 1, '__dict__', '');
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
(1, 'iPiIGGPPPt', '[us]', 1452, '', 'false', 'FLHB9HLBB'),
(2, 'VB', '[us]', 135, 'kLLkkkLLkkkLLkkkLLLLkLL', 'DRm', ''),
(3, '-', '[us]', 210, '', 'm', NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'else'),
(3, 'hero-sequel', 'Jb');
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
(1, 'Other Person', 'M''33', 3835, NULL, NULL, 'o-o', NULL, '0');
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
(1, 'Other Character', 'H1YWWHYxYWYpHYW', 1130, ' i Z333 Z', 'true', NULL),
(2, 'Other Character', 'E', 0, 'tG', '44EE', NULL),
(3, 'Voice Character', '7', 588, '_ ', 'Q%y', '0blN0');
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
(1, 'iuPJv', '__cOlIlOooF-IIl_c--u', 3, 2005, 4215, '3', NULL, 3783, NULL, 'false', 'LPT1'),
(2, 'z''8', 'else', 3, 2008, 245, 'h35r', 354, 2345, 256, 'uuud', 'Scunthorpe'),
(3, 'MhQ', '88IWW8WaNI', 2, 2011, 4637, 'HyP9PHPEly', 437, 7800, 280, '', 'RDRG');
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
(1, 1, '0', 'ojxxjmoxjxmmoxx', 'INF', 'xWWEXWX', '__dict__', 'PE');
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
(1, 3, 'NaN', 'CSCCSS', 3, 2852, 'aVVLy4O', 2601, 2047, 626, 'Y', NULL),
(2, 2, 'KKKhKKhKhKKK', '6Xnp', 1, 863, 'kLLkkkLLkkkLLkkkLLLLkLL', 74, 1942, 7861, '66666666', 'NllXNllXl--'),
(3, 3, '_W__WW00WW0_W_WW_W_W0', 'COM1', 3, NULL, '66dd66zzdz6d', 1476, 386, 1974, NULL, 'UUYU');
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
(1, 1, 2, 3, '(uncredited)', 3933, 1);
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
(1, 3, 1, 1);
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
(1, 3, 1, 1, 'NfkppkpfNokkk'),
(2, 3, 1, 1, 'dpdQtQw'),
(3, 1, 3, 2, 'NaN');
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
(1, 1, 1, '4.0', 'YF');
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
(2, 3, 3),
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
(1, 2, 1, 3),
(2, 2, 3, 3),
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
(1, 1, 1, '', 'ssNNNNN%'),
(2, 1, 1, '__dict__', '');
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
(1, 'iPiIGGPPPt', '[us]', 1452, '', 'false', 'FLHB9HLBB'),
(2, 'VB', '[us]', 135, 'm1UJT', 'DRm', ''),
(3, '-', '[us]', 210, '', 'm', NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'else'),
(3, 'hero-sequel', 'Jb');
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
(1, 'Other Person', 'M''33', 3835, NULL, NULL, 'o-o', NULL, 'm1UJT');
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
(1, 'Other Character', 'H1YWWHYxYWYpHYW', 1130, ' i Z333 Z', 'true', NULL),
(2, 'Other Character', 'E', 0, 'tG', '44EE', NULL),
(3, 'Voice Character', '7', 588, '_ ', 'Q%y', '0blN0');
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
(1, 'iuPJv', '__cOlIlOooF-IIl_c--u', 3, 2005, 4215, '3', NULL, 3783, NULL, 'false', 'LPT1'),
(2, 'z''8', 'else', 3, 2008, 245, 'h35r', 354, 2345, 256, 'uuud', 'Scunthorpe'),
(3, 'MhQ', '88IWW8WaNI', 2, 2011, 4637, 'HyP9PHPEly', 437, 7800, 280, '', 'RDRG');
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
(1, 1, '0', 'ojxxjmoxjxmmoxx', 'INF', 'xWWEXWX', '__dict__', 'PE');
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
(1, 3, 'NaN', 'CSCCSS', 3, 2852, 'aVVLy4O', 2601, 2047, 626, 'Y', NULL),
(2, 2, 'KKKhKKhKhKKK', '6Xnp', 1, 863, 'kLLkkkLLkkkLLkkkLLLLkLL', 74, 1942, 7861, '66666666', 'NllXNllXl--'),
(3, 3, '_W__WW00WW0_W_WW_W_W0', 'COM1', 3, NULL, '66dd66zzdz6d', 1476, 386, 1974, NULL, 'UUYU');
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
(1, 1, 2, 3, '(uncredited)', 3933, 1);
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
(1, 3, 1, 1);
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
(1, 3, 1, 1, 'NfkppkpfNokkk'),
(2, 3, 1, 1, 'dpdQtQw'),
(3, 1, 3, 2, 'NaN');
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
(1, 1, 1, '4.0', 'YF');
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
(2, 3, 3),
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
(1, 2, 1, 3),
(2, 2, 3, 3),
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
(1, 1, 1, '', 'ssNNNNN%'),
(2, 1, 1, '__dict__', '');
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
(1, 'iPiIGGPPPt', '[us]', 1452, '', 'false', 'FLHB9HLBB'),
(2, 'VB', '[us]', 135, 'm1UJT', 'DRm', ''),
(3, '-', '[us]', 210, '', 'm', NULL);
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
(1, 'marvel-cinematic-universe', NULL),
(2, 'hero-sequel', 'else'),
(3, 'hero-sequel', 'Jb');
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
(1, 'Other Person', 'M''33', 3835, NULL, NULL, 'o-o', NULL, '0');
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
(1, 'Other Character', 'H1YWWHYxYWYpHYW', 1130, ' i Z333 Z', 'true', NULL),
(2, 'Other Character', 'E', 0, 'tG', '44EE', NULL),
(3, 'Voice Character', '7', 588, '_ ', 'Q%y', '0blN0');
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
(1, 'iuPJv', '__cOlIlOooF-IIl_c--u', 3, 2005, 4215, '3', NULL, 3783, NULL, 'false', 'LPT1'),
(2, 'z''8', 'else', 3, 2008, 245, 'h35r', 354, 3, 256, 'uuud', 'Scunthorpe'),
(3, 'MhQ', '88IWW8WaNI', 2, 2011, 4637, 'HyP9PHPEly', 437, 7800, 280, '', 'RDRG');
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
(1, 1, '0', 'ojxxjmoxjxmmoxx', 'INF', 'xWWEXWX', '__dict__', 'PE');
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
(1, 3, 'NaN', 'CSCCSS', 3, 2852, 'aVVLy4O', 2601, 2047, 626, 'Y', NULL),
(2, 2, 'KKKhKKhKhKKK', '6Xnp', 1, 863, 'kLLkkkLLkkkLLkkkLLLLkLL', 74, 1942, 7861, '66666666', 'NllXNllXl--'),
(3, 3, '_W__WW00WW0_W_WW_W_W0', 'COM1', 3, NULL, '66dd66zzdz6d', 1476, 386, 1974, NULL, 'UUYU');
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
(1, 1, 2, 3, '(uncredited)', 3933, 1);
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
(1, 3, 1, 1);
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
(1, 3, 1, 1, 'NfkppkpfNokkk'),
(2, 3, 1, 1, 'dpdQtQw'),
(3, 1, 3, 2, 'NaN');
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
(1, 1, 1, '4.0', 'YF');
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
(2, 3, 3),
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
(1, 2, 1, 3),
(2, 2, 3, 3),
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
(1, 1, 1, '', 'ssNNNNN%'),
(2, 1, 1, '__dict__', '');
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
(1, 'oWoWWWoooWWoooooWoooWWWWWoooooWo', '[ru]', 673, 'VVVFPV', 'zzzzzzzzzzzzz', 'none'),
(2, '', '[de]', 2515, 'A77AAA', '', NULL),
(3, 'False', '[de]', NULL, 'Scunthorpe', 'undefined', 'Scunthorpe');
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
(1, 'marvel-cinematic-universe', 'true'),
(2, 'marvel-cinematic-universe', ''),
(3, 'marvel-cinematic-universe', '411aa41c3');
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
(1, 'Downey Jr., Robert', '7''''8''''''87''8', 1342, '-Infinity', 'n99nn', 'k', NULL, '77');
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
(1, 'Voice Character', '_5_e5D''', 405, '9qq', '4O', 'uuu');
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
(1, 'jm', 'Kuuk8uK0u88uupkKuu', 1, NULL, 603, '', 146, 2069, 57, '', ''),
(2, '__proto__', 'NIL', 2, 2005, 452, 'K', 558, 3521, 4220, 'JJ9', 't6W'),
(3, '333dd3d3', NULL, 1, 2010, 330, 'none', 3518, 574, 888, 'ff', '%');
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
(1, 1, 'TRUE', '0900099b0bb9Q0f', '%%%UU%UU222U%', 'True', '1e100', NULL),
(2, 1, 'qw8A', 'FaxFFxd xuux', 'xx', '0', NULL, 'if'),
(3, 1, 'LL', 'YYY', '1e100', 'uHMu YM', 'PoooXXoXXXoPoXoXPoooooPXXXPPXo', 't99t988t tt');
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
(1, 1, 'cvc', 'jjjjjjjjj', 1, 7710, '-q', 561, 339, 178, 'NULL', 'wwww');
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
(1, 1, 1, 1, '(uncredited)', 327, 2),
(2, 1, 1, NULL, '(voice) (uncredited)', 151, 1),
(3, 1, 1, 1, '(voice)', 453, 2);
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
(2, 3, 1, 1);
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
(1, 3, 2, 1, 'Inf'),
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
(1, 2, 3, 'Bulgaria', NULL),
(2, 3, 3, 'Bulgaria', NULL),
(3, 3, 3, 'Bulgaria', 'false');
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
(1, 2, 3, '4.0', 'uG8jjGuRuB'),
(2, 2, 1, '8.0', 'R'),
(3, 1, 2, '4.0', 'i');
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
(2, 3, 2),
(3, 2, 3);
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
(2, 3, 2, 2),
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
(1, 1, 2, '00', 'q'),
(2, 1, 2, 'false', 'yyyyyy'),
(3, 1, 2, 'uuuuuuuuuuuuuuuuu', 'Inf');
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
(1, 'oWoWWWoooWWoooooWoooWWWWWoooooWo', '[ru]', 673, 'VVVFPV', 'zzzzzzzzzzzzz', 'none'),
(2, '', '[de]', 2515, 'A77AAA', '', NULL),
(3, 'False', '[de]', NULL, 'Scunthorpe', 'undefined', 'Scunthorpe');
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
(1, 'marvel-cinematic-universe', 'true'),
(2, 'marvel-cinematic-universe', ''),
(3, 'marvel-cinematic-universe', '411aa41c3');
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
(1, 'Downey Jr., Robert', '7''''8''''''87''8', 1342, '-Infinity', 'n99nn', 'k', NULL, '77');
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
(1, 'Voice Character', '_5_e5D''', 405, '9qq', '4O', 'uuu');
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
(1, 'jm', 'Kuuk8uK0u88uupkKuu', 1, NULL, 603, '', 146, 2069, 57, '', ''),
(2, '__proto__', 'NIL', 2, 2005, 452, 'K', 558, 3521, 4220, 'JJ9', 't6W'),
(3, '333dd3d3', NULL, 1, 2010, 330, 'none', 3518, 574, 888, 'ff', '%');
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
(1, 1, 'TRUE', '0900099b0bb9Q0f', '%%%UU%UU222U%', 'True', '1e100', NULL),
(2, 1, 'qw8A', 'FaxFFxd xuux', 'xx', '0', NULL, 'if'),
(3, 1, 'LL', 'YYY', '1e100', 'uHMu YM', 'PoooXXoXXXoPoXoXPoooooPXXXPPXo', 't99t988t tt');
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
(1, 1, 'cvc', 'jjjjjjjjj', 1, 7710, '-q', 561, 339, 178, 'NULL', 'wwww');
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
(1, 1, 1, 1, '(uncredited)', 327, 2),
(2, 1, 1, NULL, '(voice) (uncredited)', 151, 1),
(3, 1, 1, 1, '(voice)', 453, 2);
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
(2, 3, 1, 1);
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
(1, 3, 2, 1, 'Inf'),
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
(1, 2, 3, 'Bulgaria', NULL),
(2, 3, 3, 'Bulgaria', NULL),
(3, 3, 3, 'Bulgaria', 'false');
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
(1, 2, 3, '4.0', 'uG8jjGuRuB'),
(2, 2, 1, '8.0', 'R'),
(3, 1, 2, '4.0', 'i');
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
(2, 3, 2),
(3, 2, 3);
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
(2, 3, 2, 2),
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
(1, 1, 2, '00', 'q'),
(2, 1, 2, 'false', 'yyyyyy'),
(3, 1, 2, 'uuuuuuuuuuuuuuuuu', 'Inf');
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
(1, 'oWoWWWoooWWoooooWoooWWWWWoooooWo', '[ru]', 673, 'VVVFPV', 'zzzzzzzzzzzzz', 'none'),
(2, '', '[de]', 2515, 'A77AAA', '', NULL),
(3, 'False', '[de]', NULL, 'Scunthorpe', 'undefined', 'Scunthorpe');
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
(1, 'marvel-cinematic-universe', 'true'),
(2, 'marvel-cinematic-universe', ''),
(3, 'marvel-cinematic-universe', '411aa41c3');
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
(1, 'Downey Jr., Robert', '7''''8''''''87''8', 1342, '-Infinity', 'n99nn', 'k', NULL, '77');
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
(1, 'Voice Character', '_5_e5D''', 405, '9qq', '4O', 'uuu');
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
(1, 'jm', 'Kuuk8uK0u88uupkKuu', 1, NULL, 603, '', 146, 2069, 57, '', ''),
(2, '__proto__', 'NIL', 2, 2005, 452, 'K', 558, 3521, 4220, 'JJ9', 't6W'),
(3, '333dd3d3', NULL, 1, 2010, 330, 'none', 3518, 574, 888, 'ff', '%');
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
(1, 1, 'TRUE', '0900099b0bb9Q0f', '%%%UU%UU222U%', 'Inf', '1e100', NULL),
(2, 1, 'qw8A', 'FaxFFxd xuux', 'xx', '0', NULL, 'if'),
(3, 1, 'LL', 'YYY', '1e100', 'uHMu YM', 'PoooXXoXXXoPoXoXPoooooPXXXPPXo', 't99t988t tt');
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
(1, 1, 'cvc', 'jjjjjjjjj', 1, 7710, '-q', 561, 339, 178, 'NULL', 'wwww');
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
(1, 1, 1, 1, '(uncredited)', 327, 2),
(2, 1, 1, NULL, '(voice) (uncredited)', 151, 1),
(3, 1, 1, 1, '(voice)', 453, 2);
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
(2, 3, 1, 1);
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
(1, 3, 2, 1, 'Inf'),
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
(1, 2, 3, 'Bulgaria', NULL),
(2, 3, 3, 'Bulgaria', NULL),
(3, 3, 3, 'Bulgaria', 'false');
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
(1, 2, 3, '4.0', 'uG8jjGuRuB'),
(2, 2, 1, '8.0', 'R'),
(3, 1, 2, '4.0', 'i');
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
(2, 3, 2),
(3, 2, 3);
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
(2, 3, 2, 2),
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
(1, 1, 2, '00', 'q'),
(2, 1, 2, 'false', 'yyyyyy'),
(3, 1, 2, 'uuuuuuuuuuuuuuuuu', 'Inf');
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
(1, 'oWoWWWoooWWoooooWoooWWWWWoooooWo', '[ru]', 673, 'VVVFPV', 'zzzzzzzzzzzzz', 'none'),
(2, '', '[de]', 2515, 'A77AAA', '', NULL),
(3, 'False', '[de]', NULL, 'Scunthorpe', 'undefined', 'Scunthorpe');
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
(1, 'marvel-cinematic-universe', 'true'),
(2, 'marvel-cinematic-universe', ''),
(3, 'marvel-cinematic-universe', '411aa41c3');
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
(1, 'Downey Jr., Robert', '7''''8''''''87''8', 1342, '-Infinity', 'n99nn', 'k', NULL, 'none');
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
(1, 'Voice Character', '_5_e5D''', 405, '9qq', '4O', 'uuu');
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
(1, 'jm', 'Kuuk8uK0u88uupkKuu', 1, NULL, 603, '', 146, 2069, 57, '', ''),
(2, '__proto__', 'NIL', 2, 2005, 452, 'K', 558, 3521, 4220, 'JJ9', 't6W'),
(3, '333dd3d3', NULL, 1, 2010, 330, 'none', 3518, 574, 888, 'ff', '%');
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
(1, 1, 'TRUE', '0900099b0bb9Q0f', '%%%UU%UU222U%', 'True', '1e100', NULL),
(2, 1, 'qw8A', 'FaxFFxd xuux', 'xx', '0', NULL, 'if'),
(3, 1, 'LL', 'YYY', '1e100', 'uHMu YM', 'PoooXXoXXXoPoXoXPoooooPXXXPPXo', 't99t988t tt');
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
(1, 1, 'cvc', 'jjjjjjjjj', 1, 7710, '-q', 561, 339, 178, 'NULL', 'wwww');
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
(1, 1, 1, 1, '(uncredited)', 327, 2),
(2, 1, 1, NULL, '(voice) (uncredited)', 151, 1),
(3, 1, 1, 1, '(voice)', 453, 2);
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
(2, 3, 1, 1);
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
(1, 3, 2, 1, 'Inf'),
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
(1, 2, 3, 'Bulgaria', NULL),
(2, 3, 3, 'Bulgaria', NULL),
(3, 3, 3, 'Bulgaria', 'false');
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
(1, 2, 3, '4.0', 'uG8jjGuRuB'),
(2, 2, 1, '8.0', 'R'),
(3, 1, 2, '4.0', 'i');
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
(2, 3, 2),
(3, 2, 3);
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
(2, 3, 2, 2),
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
(1, 1, 2, '00', 'q'),
(2, 1, 2, 'false', 'yyyyyy'),
(3, 1, 2, 'uuuuuuuuuuuuuuuuu', 'Inf');
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
(1, '999999999999999999999999999999', '[us]', 32, NULL, 'None', '5ppPP5p5Pppp');
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
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', 'eee');
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
(1, 'Other Person', 'NULL', 3378, NULL, 'VvtVvS', '', '12', 'LLIK2KKI'),
(2, 'Other Person', 'C5', 114, '', 'D', '', NULL, 'PPPPPP');
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
(1, 'Voice Character', '', 64, 'o', 'q4', 'mw'),
(2, 'Other Character', 'vvlJ', 255, NULL, NULL, 'False');
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
(1, 'NNKMKNKMMNlNMNMM', 'N6', 2, 2009, 8191, 'pppp', NULL, 256, 512, 'none', 'Ai-ZpiuuijAZjjpZlAuZli'),
(2, 'B', NULL, 3, 2012, 256, 'INF', 511, 1308, 650, '--Goo-k4o', '-Infinity'),
(3, 'BBRRRBBRBR', '%U4g4U%4UU44v', 2, 2012, 398, 'jy', 1023, 7, 128, '0', 'pXpppX');
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
(1, 1, 'KK', 'S', NULL, 'hW', 'V4Dy', 'w'''),
(2, 1, '11AAAOA%%', '', 'DDDDD', 'ptkR8t', NULL, 'NIL');
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
(1, 3, 'U', '999999999999999999999999999999', 2, 128, '', 298, 1, 3, 'HH%%%HHH%HH%%%%H%%%%H%H%%%%HH', '0'),
(2, 3, 'zzFFmzmccOmzO', 'NULL', 1, NULL, NULL, 4096, 1, 0, 'D9fS', '3');
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
(1, 1, 2, 1, '(voice)', 5, 1);
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
(1, 3, 1, 2),
(2, 3, 1, 1),
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
(1, 1, 1, 2, 'Xg');
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
(2, 1, 1, 'USA', 'then');
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
(1, 2, 1, '4.0', '0'),
(2, 3, 1, '8.0', 'DSk''');
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
(1, 1, 1, 'ikgkk''ppk''bpkbpkbxbgkgx''kb', 'NUL');
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
(1, '999999999999999999999999999999', '[us]', 32, NULL, 'None', '5ppPP5p5Pppp');
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
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', 'eee');
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
(1, 'Other Person', 'NULL', 3378, NULL, 'VvtVvS', '', '12', 'LLIK2KKI'),
(2, 'Other Person', 'C5', 114, '', 'D', '', NULL, 'PPPPPP');
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
(1, 'Voice Character', '', 64, 'o', 'q4', 'mw'),
(2, 'Other Character', 'vvlJ', 255, NULL, NULL, 'False');
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
(1, 'NNKMKNKMMNlNMNMM', 'N6', 2, 2009, 8191, 'pppp', NULL, 256, 512, 'none', 'Ai-ZpiuuijAZjjpZlAuZli'),
(2, 'B', NULL, 3, 2012, 256, 'INF', 511, 1308, 650, '--Goo-k4o', '-Infinity'),
(3, 'BBRRRBBRBR', '%U4g4U%4UU44v', 2, 2012, 398, 'jy', 1023, 7, 128, '0', 'pXpppX');
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
(1, 1, 'KK', 'S', NULL, 'hW', 'V4Dy', 'w'''),
(2, 1, '11AAAOA%%', '', 'DDDDD', 'ptkR8t', NULL, 'NIL');
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
(1, 3, 'U', '999999999999999999999999999999', 2, 128, '', 298, 1, 3, 'HH%%%HHH%HH%%%%H%%%%H%H%%%%HH', '0'),
(2, 3, '', NULL, 2, NULL, NULL, NULL, NULL, NULL, '1', NULL);
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
(1, 2, 2, 2, NULL, 1, 1);
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
(2, NULL, 2, 1),
(3, 2, 1, 2);
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
(1, 3, 1, 2, '1');
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
(1, 2, 1, '4.0', NULL),
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
(2, 2, 2),
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
(1, 1, 1, '', NULL);
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
(1, '999999999999999999999999999999', '[us]', 32, NULL, 'None', '5ppPP5p5Pppp');
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
(1, 'character-name-in-title', NULL),
(2, 'hero-sequel', 'eee');
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
(1, 'Other Person', 'NULL', 3378, NULL, 'VvtVvS', '', '12', 'LLIK2KKI'),
(2, 'Other Person', 'C5', 114, '', 'D', '', NULL, 'PPPPPP');
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
(1, 'Voice Character', '', 64, 'o', 'q4', 'mw'),
(2, 'Other Character', 'vvlJ', 255, NULL, NULL, 'none');
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
(1, 'NNKMKNKMMNlNMNMM', 'N6', 2, 2009, 8191, 'pppp', NULL, 256, 512, 'none', 'Ai-ZpiuuijAZjjpZlAuZli'),
(2, 'B', NULL, 3, 2012, 256, 'INF', 511, 1308, 650, '--Goo-k4o', '-Infinity'),
(3, 'BBRRRBBRBR', '%U4g4U%4UU44v', 2, 2012, 398, 'jy', 1023, 7, 128, '0', 'pXpppX');
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
(1, 1, 'KK', 'S', NULL, 'hW', 'V4Dy', 'w'''),
(2, 1, '11AAAOA%%', '', 'DDDDD', 'ptkR8t', NULL, 'NIL');
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
(1, 3, 'U', '999999999999999999999999999999', 2, 128, '', 298, 1, 3, 'HH%%%HHH%HH%%%%H%%%%H%H%%%%HH', '0'),
(2, 3, 'zzFFmzmccOmzO', 'NULL', 1, NULL, NULL, 4096, 1, 0, 'D9fS', '3');
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
(1, 1, 2, 1, '(voice)', 5, 1);
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
(1, 3, 1, 2),
(2, 3, 1, 1),
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
(1, 1, 1, 2, 'Xg');
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
(2, 1, 1, 'USA', 'then');
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
(1, 2, 1, '4.0', '0'),
(2, 3, 1, '8.0', 'DSk''');
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
(1, 1, 1, 'ikgkk''ppk''bpkbpkbxbgkgx''kb', 'NUL');
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
(1, '-t-', '[us]', 19, 'OF%', 'Scunthorpe', 'FALSE');
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
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', 'qmmqqPv'),
(3, 'marvel-cinematic-universe', 'ggDZZEDDS55D');
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
(1, 'Other Person', 'none', 1023, '0', 'null', 'then', 'Onnn_O_n__nOnn', 'True'),
(2, 'Other Person', 't4tttt', 1239, '_zsyyaa_a', '__proto__', 'QQQQQ', '', 'nil');
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
(1, 'Other Character', 'k', 3184, 'l', '', 'None'),
(2, 'Voice Character', 'Vucq', 4096, '', 'IA2AFTTF2TTFIFFAFAF', '3ll3l33'),
(3, 'Voice Character', 'KKXC', 0, NULL, '', 'FALSE');
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
(1, 'cpsySySeeS', 'NIL', 2, 2005, 115, '', 37, NULL, 133, 'U UXFUUF', 'gZZ0gg4');
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
(1, 2, '-Infinity', 'UUUUUUUUUUUUUUUUU', NULL, '1eOQa', 'ikD0D', 'Q%K-KuK');
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
(1, 1, 'LPT1', '-Infinity', 2, 2105, NULL, 3441, 852, 295, 'lB', 'eMEM');
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
(1, 1, 1, 3, '(voice) (uncredited)', 1218, 2);
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
(1, 1, 1, 1, 'N71r7N11im'),
(2, 1, 1, 1, 'F');
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
(1, 1, 2, 'USA', ''),
(2, 1, 1, 'Bulgaria', 'G');
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
(1, 1, 2, '8.0', '999999999999999999999999999999'),
(2, 1, 3, '8.0', 'true'),
(3, 1, 1, '8.0', '');
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
(1, 2, 1, '', '55'),
(2, 1, 2, '9d9d', 'DDgXgkkXdDdddgXgXX'),
(3, 1, 1, '6P%y', NULL);
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
(1, '-t-', '[us]', 19, 'OF%', 'Scunthorpe', 'FALSE');
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
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', 'qmmqqPv'),
(3, 'marvel-cinematic-universe', 'ggDZZEDDS55D');
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
(1, 'Other Person', 'none', 1023, '0', 'null', 'then', 'Onnn_O_n__nOnn', 'True'),
(2, 'Other Person', 't4tttt', 1239, '_zsyyaa_a', '__proto__', 'QQQQQ', '', 'nil');
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
(1, 'Other Character', 'k', NULL, NULL, 'l', ''),
(2, 'Other Character', NULL, NULL, NULL, 'Vucq', '1'),
(3, 'Other Character', 'IA2AFTTF2TTFIFFAFAF', 1, NULL, NULL, 'KKXC');
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
(1, '1', NULL, 2, 2006, 1, NULL, NULL, 1, NULL, NULL, '1');
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
(1, 2, '', '', NULL, NULL, '1', NULL);
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
(1, 1, '', NULL, 2, NULL, NULL, NULL, 1, NULL, NULL, '1eOQa');
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
(1, 2, 1, NULL, '(voice)', NULL, 1);
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
(1, 1, 1, 2, '1'),
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
(1, 1, 2, 'USA', NULL),
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
(1, 1, 1, '4.0', NULL),
(2, 1, 2, '4.0', NULL),
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
(1, 2, 2, '1', NULL),
(2, 1, 2, '', NULL),
(3, 1, 2, '', NULL);
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
(1, '-t-', '[us]', 19, 'OF%', 'Scunthorpe', 'FALSE');
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
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', 'qmmqqPv'),
(3, 'marvel-cinematic-universe', 'ggDZZEDDS55D');
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
(1, 'Downey Jr., Robert', NULL, 1, NULL, '1', NULL, NULL, 'null'),
(2, 'Other Person', NULL, NULL, 'Onnn_O_n__nOnn', 'True', NULL, 't4tttt', '1');
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
(1, 'Other Character', NULL, 1, NULL, '', 'nil'),
(2, 'Other Character', NULL, NULL, NULL, 'k', '1'),
(3, 'Other Character', NULL, 1, 'None', NULL, 'Vucq');
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
(1, '1', NULL, 1, 2006, NULL, NULL, 1, NULL, NULL, 'KKXC', '1');
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
(1, 2, '', 'FALSE', NULL, 'NIL', NULL, '1');
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
(1, 1, '', '', 1, NULL, '1', NULL, NULL, 1, NULL, NULL);
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
(1, 1, 2, 'USA', NULL),
(2, 1, 2, 'USA', '1');
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
(1, 1, 2, '4.0', 'lB'),
(2, 1, 2, '4.0', NULL),
(3, 1, 2, '4.0', '1');
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
(1, 2, 2, '1', NULL),
(2, 2, 2, '1', NULL),
(3, 1, 2, '', NULL);
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
(1, '-t-', '[us]', 19, 'OF%', 'Scunthorpe', 'FALSE');
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
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', 'qmmqqPv'),
(3, 'marvel-cinematic-universe', 'ggDZZEDDS55D');
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
(1, 'Other Person', 'none', 1023, '0', 'null', 'then', 'Onnn_O_n__nOnn', 'True'),
(2, 'Other Person', 't4tttt', 1239, '_zsyyaa_a', '__proto__', 'QQQQQ', '', 'nil');
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
(1, 'Other Character', 'k', 3184, 'None', '', 'None'),
(2, 'Voice Character', 'Vucq', 4096, '', 'IA2AFTTF2TTFIFFAFAF', '3ll3l33'),
(3, 'Voice Character', 'KKXC', 0, NULL, '', 'FALSE');
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
(1, 'cpsySySeeS', 'NIL', 2, 2005, 115, '', 37, NULL, 133, 'U UXFUUF', 'gZZ0gg4');
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
(1, 2, '-Infinity', 'UUUUUUUUUUUUUUUUU', NULL, '1eOQa', 'ikD0D', 'Q%K-KuK');
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
(1, 1, 'LPT1', '-Infinity', 2, 2105, NULL, 3441, 852, 295, 'lB', 'eMEM');
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
(1, 1, 1, 3, '(voice) (uncredited)', 1218, 2);
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
(1, 1, 1, 1, 'N71r7N11im'),
(2, 1, 1, 1, 'F');
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
(1, 1, 2, 'USA', ''),
(2, 1, 1, 'Bulgaria', 'G');
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
(1, 1, 2, '8.0', '999999999999999999999999999999'),
(2, 1, 3, '8.0', 'true'),
(3, 1, 1, '8.0', '');
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
(1, 2, 1, '', '55'),
(2, 1, 2, '9d9d', 'DDgXgkkXdDdddgXgXX'),
(3, 1, 1, '6P%y', NULL);
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
(1, '-t-', '[us]', 19, 'OF%', 'Scunthorpe', 'FALSE');
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
(3, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', NULL),
(2, 'marvel-cinematic-universe', 'qmmqqPv'),
(3, 'marvel-cinematic-universe', 'ggDZZEDDS55D');
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
(1, 'Other Person', 'none', 1023, '0', 'null', 'then', 'Onnn_O_n__nOnn', 'True'),
(2, 'Other Person', 't4tttt', 1239, '_zsyyaa_a', '__proto__', 'QQQQQ', '', 'nil');
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
(1, 'Other Character', 'k', 3184, 'l', '', 'None'),
(2, 'Voice Character', 'Vucq', 4096, '', 'IA2AFTTF2TTFIFFAFAF', '3ll3l33'),
(3, 'Voice Character', 'KKXC', 0, NULL, '', 'FALSE');
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
(1, 'cpsySySeeS', 'NIL', 2, 2005, 115, '', 37, NULL, 133, 'U UXFUUF', 'gZZ0gg4');
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
(1, 2, '-Infinity', 'UUUUUUUUUUUUUUUUU', NULL, '1eOQa', 'ikD0D', 'Q%K-KuK');
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
(1, 1, 'LPT1', '-Infinity', 2, 2105, NULL, 3441, 852, 295, 'lB', 'eMEM');
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
(1, 1, 1, 3, '(voice) (uncredited)', 1218, 2);
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
(2, 1, 2, 'USA', '');
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
(1, 1, 1, '8.0', 'G'),
(2, 1, 2, '8.0', '999999999999999999999999999999'),
(3, 1, 3, '8.0', 'true');
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
(1, 1, 2, '1', NULL),
(2, 2, 1, '', '55'),
(3, 1, 2, '9d9d', 'DDgXgkkXdDdddgXgXX');
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
(1, 'complete+verified'),
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
(1, 'Scunthorpe', '[de]', 3779, 'BBBhhhB', '0MM4M4cMf4Xy0', 'D33ZDBKB3');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'NaN');
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
(1, 'Downey Jr., Robert', 'FALSE', 129, '5H5HE', '', 'dd', '0', 'NIL'),
(2, 'Downey Jr., Robert', 'e', 882, '00', 'Scunthorpe', 'None', 'NUL', '1e100'),
(3, 'Other Person', '', 1447, 'wTwq2w2ggw', 'h', 'IIttSI', '', '5555 5 Rk5k R');
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
(1, 'Other Character', 'QPI_IQQ', 8650, 'vRvv', 'nil', '__proto__');
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
(1, '4i4', 'M', 1, 2011, 99, 'true', 2048, 65, 6942, '0', 'NaN'),
(2, '0', 'WK4', 1, 2012, NULL, 'vP', NULL, 4253, 238, 'SSkkkk', 'if');
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
(1, 1, 'undefined', 'XXXXXXXXXXXXXXXXXXXXXXXXX', '', '4VFFT4T4TWW4TW', '', 'Xo2Su2'),
(2, 1, 'INF', 'ZZZ', 'o36TK3K633', 'vmvmmv22m22m22mv22v2v2vmvvvmmm', 'M44GwG4', 'nil');
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
(1, 2, '', 'sfuQsQs', 1, NULL, 'LH', 139, 305, 678, 'uueEO', ''),
(2, 1, 'W', '999999999999999999999999999999', 1, 7, 'INF', 2047, 140, 1109, 'nddqqcdqnIIn', 'VVV');
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
(1, 1, 1, 1, '(uncredited)', 1164, 1);
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
(1, 2, 1, 2),
(2, 1, 3, 3),
(3, 1, 2, 3);
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
(1, 2, 1, 1, 'hDtVYc'),
(2, 1, 1, 1, 'lNNYWzWWZllzlZ');
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
(1, 1, 1, '4.0', 'slxsc'),
(2, 1, 1, '8.0', 'kKkk');
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
(1, 3, 1, 'hhhhhhhhhhhhh', NULL),
(2, 2, 2, '', 'None');
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
(1, 'Scunthorpe', '[de]', 3779, 'BBBhhhB', '0MM4M4cMf4Xy0', 'D33ZDBKB3');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'NaN');
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
(1, 'Downey Jr., Robert', 'FALSE', 129, '5H5HE', '', 'dd', '0', 'NIL'),
(2, 'Downey Jr., Robert', 'e', 882, '00', 'Scunthorpe', 'None', 'NUL', '1e100'),
(3, 'Other Person', '', 1447, 'wTwq2w2ggw', 'h', 'IIttSI', '', '5555 5 Rk5k R');
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
(1, 'Other Character', 'QPI_IQQ', 8650, 'vRvv', 'nil', '__proto__');
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
(1, '4i4', 'M', 1, 2011, 99, 'true', 2048, 65, 6942, '0', 'NaN'),
(2, '0', 'WK4', 1, 2012, NULL, 'vP', NULL, 4253, 238, 'SSkkkk', 'if');
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
(1, 1, 'wTwq2w2ggw', 'XXXXXXXXXXXXXXXXXXXXXXXXX', '', '4VFFT4T4TWW4TW', '', 'Xo2Su2'),
(2, 1, 'INF', 'ZZZ', 'o36TK3K633', 'vmvmmv22m22m22mv22v2v2vmvvvmmm', 'M44GwG4', 'nil');
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
(1, 2, '', 'sfuQsQs', 1, NULL, 'LH', 139, 305, 678, 'uueEO', ''),
(2, 1, 'W', '999999999999999999999999999999', 1, 7, 'INF', 2047, 140, 1109, 'nddqqcdqnIIn', 'VVV');
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
(1, 1, 1, 1, '(uncredited)', 1164, 1);
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
(1, 2, 1, 2),
(2, 1, 3, 3),
(3, 1, 2, 3);
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
(1, 2, 1, 1, 'hDtVYc'),
(2, 1, 1, 1, 'lNNYWzWWZllzlZ');
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
(1, 1, 1, '4.0', 'slxsc'),
(2, 1, 1, '8.0', 'kKkk');
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
(1, 3, 1, 'hhhhhhhhhhhhh', NULL),
(2, 2, 2, '', 'None');
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
(1, 'Scunthorpe', '[de]', 3779, 'BBBhhhB', '0MM4M4cMf4Xy0', 'D33ZDBKB3');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'NaN');
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
(1, 'Downey Jr., Robert', 'FALSE', 129, '5H5HE', '', 'dd', '0', 'NIL'),
(2, 'Downey Jr., Robert', 'e', 882, '00', 'Scunthorpe', 'None', 'NUL', '1e100'),
(3, 'Other Person', '', 1447, 'wTwq2w2ggw', 'h', 'IIttSI', '', '5555 5 Rk5k R');
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
(1, 'Other Character', 'QPI_IQQ', 8650, 'vRvv', 'nil', '__proto__');
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
(1, '4i4', 'M', 1, 2011, 99, 'true', 2048, 65, 6942, '5555 5 Rk5k R', 'NaN'),
(2, '0', 'WK4', 1, 2012, NULL, 'vP', NULL, 4253, 238, 'SSkkkk', 'if');
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
(1, 1, 'undefined', 'XXXXXXXXXXXXXXXXXXXXXXXXX', '', '4VFFT4T4TWW4TW', '', 'Xo2Su2'),
(2, 1, 'INF', 'ZZZ', 'o36TK3K633', 'vmvmmv22m22m22mv22v2v2vmvvvmmm', 'M44GwG4', 'nil');
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
(1, 2, '', 'sfuQsQs', 1, NULL, 'LH', 139, 305, 678, 'uueEO', ''),
(2, 1, 'W', '999999999999999999999999999999', 1, 7, 'INF', 2047, 140, 1109, 'nddqqcdqnIIn', 'VVV');
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
(1, 1, 1, 1, '(uncredited)', 1164, 1);
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
(1, 2, 1, 2),
(2, 1, 3, 3),
(3, 1, 2, 3);
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
(1, 2, 1, 1, 'hDtVYc'),
(2, 1, 1, 1, 'lNNYWzWWZllzlZ');
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
(1, 1, 1, '4.0', 'slxsc'),
(2, 1, 1, '8.0', 'kKkk');
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
(1, 3, 1, 'hhhhhhhhhhhhh', NULL),
(2, 2, 2, '', 'None');
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
(1, 'Scunthorpe', '[de]', 3779, 'BBBhhhB', '0MM4M4cMf4Xy0', 'D33ZDBKB3');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'NaN');
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
(1, 'Downey Jr., Robert', 'FALSE', 129, '5H5HE', '', 'dd', '0', '__proto__'),
(2, 'Downey Jr., Robert', 'e', 882, '00', 'Scunthorpe', 'None', 'NUL', '1e100'),
(3, 'Other Person', '', 1447, 'wTwq2w2ggw', 'h', 'IIttSI', '', '5555 5 Rk5k R');
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
(1, 'Other Character', 'QPI_IQQ', 8650, 'vRvv', 'nil', '__proto__');
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
(1, '4i4', 'M', 1, 2011, 99, 'true', 2048, 65, 6942, '0', 'NaN'),
(2, '0', 'WK4', 1, 2012, NULL, 'vP', NULL, 4253, 238, 'SSkkkk', 'if');
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
(1, 1, 'undefined', 'XXXXXXXXXXXXXXXXXXXXXXXXX', '', '4VFFT4T4TWW4TW', '', 'Xo2Su2'),
(2, 1, 'INF', 'ZZZ', 'o36TK3K633', 'vmvmmv22m22m22mv22v2v2vmvvvmmm', 'M44GwG4', 'nil');
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
(1, 2, '', 'sfuQsQs', 1, NULL, 'LH', 139, 305, 678, 'uueEO', ''),
(2, 1, 'W', '999999999999999999999999999999', 1, 7, 'INF', 2047, 140, 1109, 'nddqqcdqnIIn', 'VVV');
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
(1, 1, 1, 1, '(uncredited)', 1164, 1);
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
(1, 2, 1, 2),
(2, 1, 3, 3),
(3, 1, 2, 3);
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
(1, 2, 1, 1, 'hDtVYc'),
(2, 1, 1, 1, 'lNNYWzWWZllzlZ');
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
(1, 1, 1, '4.0', 'slxsc'),
(2, 1, 1, '8.0', 'kKkk');
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
(1, 3, 1, 'hhhhhhhhhhhhh', NULL),
(2, 2, 2, '', 'None');
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
(1, 'Scunthorpe', '[de]', 3779, 'BBBhhhB', '0MM4M4cMf4Xy0', 'D33ZDBKB3');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'hero-sequel', 'NaN');
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
(1, 'Downey Jr., Robert', 'FALSE', 129, '5H5HE', '', 'dd', '0', 'NIL'),
(2, 'Downey Jr., Robert', 'e', 882, '00', 'Scunthorpe', 'None', 'NUL', '1e100'),
(3, 'Other Person', '', 1447, 'wTwq2w2ggw', 'h', 'IIttSI', '', '5555 5 Rk5k R');
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
(1, 'Other Character', 'QPI_IQQ', 8650, 'vRvv', 'nil', '__proto__');
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
(1, '4i4', 'M', 1, 2011, 99, 'true', 2048, 65, 6942, '0', 'NaN'),
(2, '0', 'WK4', 1, 2006, NULL, 'vP', NULL, 4253, 238, 'SSkkkk', 'if');
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
(1, 1, 'undefined', 'XXXXXXXXXXXXXXXXXXXXXXXXX', '', '4VFFT4T4TWW4TW', '', 'Xo2Su2'),
(2, 1, 'INF', 'ZZZ', 'o36TK3K633', 'vmvmmv22m22m22mv22v2v2vmvvvmmm', 'M44GwG4', 'nil');
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
(1, 2, '', 'sfuQsQs', 1, NULL, 'LH', 139, 305, 678, 'uueEO', ''),
(2, 1, 'W', '999999999999999999999999999999', 1, 7, 'INF', 2047, 140, 1109, 'nddqqcdqnIIn', 'VVV');
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
(1, 1, 1, 1, '(uncredited)', 1164, 1);
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
(1, 2, 1, 2),
(2, 1, 3, 3),
(3, 1, 2, 3);
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
(1, 2, 1, 1, 'hDtVYc'),
(2, 1, 1, 1, 'lNNYWzWWZllzlZ');
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
(1, 1, 1, '4.0', 'slxsc'),
(2, 1, 1, '8.0', 'kKkk');
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
(1, 3, 1, 'hhhhhhhhhhhhh', NULL),
(2, 2, 2, '', 'None');
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
(1, 'FALSE', '[us]', 368, NULL, 'NIL', ''),
(2, 'Sz', '[ru]', NULL, 'NaN', 'then', 'VVV'),
(3, 'eUv_eU__', '[ru]', 986, '0', '', 'True');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'SDbD');
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
(1, 'Other Person', NULL, 1569, 'VRVYgQ-ezwzw-eg---URwU-', NULL, 'oMo-wwYw', '', '');
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
(1, 'Voice Character', 'NaN', NULL, 'y9Cyo', '1pp11pr1prrprrp', 'MLVdVL1m'),
(2, 'Other Character', '', 7463, '8G', '99', 'Bu09YY');
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
(1, 'kZvZ1P', '3G', 1, 2006, 4375, 'uujsEuEcE8j', 2400, 63, 2215, 'UJ1JJl1JJJJ1UJU3JJJUpJUbJU1l3b', NULL);
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
(1, 1, 'CkTkCCT', '', 'V3dVjV', '1e100', 'TTTss8T88', NULL),
(2, 1, 'VugVLggLLLpu', '-Infinity', '4''''''''''44''', NULL, '11F', 'yu'),
(3, 1, '2W', 'i', 'Yhh', '', 'b-P--PbP-PP', '');
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
(1, 1, 'VlThllVh1', 'RbKR', 2, NULL, 'MDM', 7734, 831, 190, 'uHgH9HgguuH99u9IgIuHgH9Hu9H9Ig9I', ''),
(2, 1, 'COM1', 'False', 3, 488, '888888888888888888', 4, NULL, 60, 'YYYYYYYYYYYYY', NULL),
(3, 1, 'KKx', 'll', 2, 2893, 'HRR6aH6HRv', NULL, 511, 3926, '_', '666666666666666666');
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
(1, 1, 1, 2, NULL, NULL, 1);
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
(1, 1, 2, 2);
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
(2, 1, 1, 2, 'fw7wg_wG_GGG__qq'),
(3, 1, 3, 1, 'FALSE');
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
(1, 1, 2, 'USA', 'False'),
(2, 1, 2, 'Bulgaria', '');
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
(1, 1, 1, '8.0', 'VL'),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '4.0', 'NIL');
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
(1, 1, 2, 'NULL', '''ls7ls'),
(2, 1, 2, 'nn', 'Q  tt'),
(3, 1, 1, '00', '');
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
(1, 'FALSE', '[us]', 368, NULL, 'NIL', ''),
(2, 'Sz', '[ru]', NULL, 'NaN', 'then', 'VVV'),
(3, 'eUv_eU__', '[ru]', 986, '0', '', 'True');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'SDbD');
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
(1, 'Other Person', NULL, 1569, 'VRVYgQ-ezwzw-eg---URwU-', NULL, 'oMo-wwYw', '', '');
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
(1, 'Voice Character', 'NaN', NULL, 'y9Cyo', '1pp11pr1prrprrp', 'MLVdVL1m'),
(2, 'Other Character', '', 7463, '8G', '99', 'Bu09YY');
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
(1, 'kZvZ1P', '3G', 1, 2006, 4375, 'uujsEuEcE8j', 2400, 63, 2215, 'UJ1JJl1JJJJ1UJU3JJJUpJUbJU1l3b', NULL);
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
(1, 1, 'CkTkCCT', '', 'V3dVjV', '1e100', 'TTTss8T88', NULL),
(2, 1, 'VugVLggLLLpu', '-Infinity', '4''''''''''44''', NULL, '11F', 'yu'),
(3, 1, '2W', 'i', 'Yhh', '', 'b-P--PbP-PP', '');
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
(1, 1, 'VlThllVh1', '', 2, NULL, NULL, 1, NULL, 7734, '1', NULL),
(2, 1, 'uHgH9HgguuH99u9IgIuHgH9Hu9H9Ig9I', '', 1, NULL, NULL, 1, NULL, NULL, '1', NULL),
(3, 1, '', NULL, 2, 60, 'YYYYYYYYYYYYY', NULL, NULL, NULL, NULL, 'll');
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
(1, 1, 1, NULL, '(voice)', NULL, 1);
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
(1, 1, 2, 2);
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
(1, 1, 2, 2, '666666666666666666'),
(2, 1, 1, 2, NULL),
(3, 1, 2, 1, '1');
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
(1, 1, 2, '4.0', NULL),
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
(1, 1, 1, '1', 'VL'),
(2, 1, 1, '1', NULL),
(3, 1, 1, '', NULL);
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
(1, 'FALSE', '[us]', 3, NULL, 'NIL', ''),
(2, 'Sz', '[ru]', NULL, 'NaN', 'then', 'VVV'),
(3, 'eUv_eU__', '[ru]', 986, '0', '', 'True');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'SDbD');
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
(1, 'Other Person', NULL, 1569, 'VRVYgQ-ezwzw-eg---URwU-', NULL, 'oMo-wwYw', '', '');
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
(1, 'Voice Character', 'NaN', NULL, 'y9Cyo', '1pp11pr1prrprrp', 'MLVdVL1m'),
(2, 'Other Character', '', 7463, '8G', '99', 'Bu09YY');
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
(1, 'kZvZ1P', '3G', 1, 2006, 4375, 'uujsEuEcE8j', 2400, 63, 2215, 'UJ1JJl1JJJJ1UJU3JJJUpJUbJU1l3b', NULL);
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
(1, 1, 'CkTkCCT', '', 'V3dVjV', '1e100', 'TTTss8T88', NULL),
(2, 1, 'VugVLggLLLpu', '-Infinity', '4''''''''''44''', NULL, '11F', 'yu'),
(3, 1, '2W', 'i', 'Yhh', '', 'b-P--PbP-PP', '');
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
(1, 1, 'VlThllVh1', 'RbKR', 2, NULL, 'MDM', 7734, 831, 190, 'uHgH9HgguuH99u9IgIuHgH9Hu9H9Ig9I', ''),
(2, 1, 'COM1', 'False', 3, 488, '888888888888888888', 4, NULL, 60, 'YYYYYYYYYYYYY', NULL),
(3, 1, 'KKx', 'll', 2, 2893, 'HRR6aH6HRv', NULL, 511, 3926, '_', '666666666666666666');
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
(1, 1, 1, 2, NULL, NULL, 1);
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
(1, 1, 2, 2);
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
(2, 1, 1, 2, 'fw7wg_wG_GGG__qq'),
(3, 1, 3, 1, 'FALSE');
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
(1, 1, 2, 'USA', 'False'),
(2, 1, 2, 'Bulgaria', '');
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
(1, 1, 1, '8.0', 'VL'),
(2, 1, 1, '4.0', NULL),
(3, 1, 1, '4.0', 'NIL');
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
(1, 1, 2, 'NULL', '''ls7ls'),
(2, 1, 2, 'nn', 'Q  tt'),
(3, 1, 1, '00', '');
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
(1, 'FALSE', '[us]', 368, NULL, 'NIL', ''),
(2, 'Sz', '[ru]', NULL, 'NaN', 'then', 'VVV'),
(3, 'eUv_eU__', '[ru]', 986, '0', '', 'True');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'SDbD');
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
(1, 'Other Person', NULL, 1569, 'VRVYgQ-ezwzw-eg---URwU-', NULL, 'oMo-wwYw', '', '');
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
(1, 'Voice Character', 'NaN', NULL, 'y9Cyo', '1pp11pr1prrprrp', 'MLVdVL1m'),
(2, 'Other Character', '', 7463, '8G', '99', 'Bu09YY');
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
(1, 'kZvZ1P', '3G', 1, 2006, 4375, 'uujsEuEcE8j', 2400, 63, 2215, 'UJ1JJl1JJJJ1UJU3JJJUpJUbJU1l3b', NULL);
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
(1, 1, 'CkTkCCT', '', 'V3dVjV', '1e100', 'TTTss8T88', NULL),
(2, 1, 'VugVLggLLLpu', '-Infinity', '4''''''''''44''', NULL, '11F', 'yu'),
(3, 1, '2W', 'i', 'Yhh', '', 'b-P--PbP-PP', '');
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
(1, 1, 'VlThllVh1', 'RbKR', 2, NULL, 'MDM', 7734, 831, 190, 'uHgH9HgguuH99u9IgIuHgH9Hu9H9Ig9I', ''),
(2, 1, 'COM1', 'False', 3, 488, '888888888888888888', 4, NULL, 60, 'YYYYYYYYYYYYY', NULL),
(3, 1, 'KKx', 'll', 2, 2893, 'HRR6aH6HRv', NULL, 511, 3926, '_', '666666666666666666');
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
(1, 1, 1, 2, NULL, NULL, 1);
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
(1, 1, 2, 2);
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
(2, 1, 1, 2, 'fw7wg_wG_GGG__qq'),
(3, 1, 3, 1, 'FALSE');
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
(1, 1, 2, 'USA', 'False'),
(2, 1, 2, 'Bulgaria', '');
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
(1, 1, 1, '8.0', 'VL'),
(2, 1, 1, '4.0', NULL),
(3, 1, 2, '4.0', 'NIL');
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
(1, 1, 2, 'NULL', '''ls7ls'),
(2, 1, 2, 'nn', 'Q  tt'),
(3, 1, 1, '00', '');
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
(1, 'True', '[us]', NULL, 'IIIIIhI', 'false', NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'GG'),
(2, 'hero-sequel', 'hhhhh0G'),
(3, 'marvel-cinematic-universe', 'FooF');
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
(1, 'Downey Jr., Robert', 'vMvMrErrvM', 63, '9j9jjjjb9ojboo', '4', 'riErE', 'NIL', 'tDi'),
(2, 'Downey Jr., Robert', 'false', 4096, 'then', 'Scunthorpe', 'True', 'TRUE', 'then');
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
(1, 'Voice Character', NULL, 2666, 'aa5a5aaaa55a555a5a5', NULL, NULL),
(2, 'Voice Character', '', 494, 'ZZZ', '5wgBWBSW', '0'),
(3, 'Other Character', '', 98, '', 'M', '1e100');
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
(1, '__proto__', '1Yt', 1, 2012, 1102, 'LPT1', 128, 255, 127, '1e100', 'Inf'),
(2, 'JJJJJJ', 'sX_ic', 1, 2007, 16, 'TRUE', 16, 1, 6, 'zt', 'V'),
(3, 'undefined', '%%y%', 1, 2006, 255, 'nil', 2, 717, 3565, 'False', '0SCy0CyUCUiyCU');
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
(1, 2, 'cY', 'iiii', NULL, 'XuXllulluXX', 'Scunthorpe', '00'),
(2, 1, '', 'SpDpdpPh4pdaD', 'None', 'C_', 'hbhbhhhhhhbh', '1');
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
(1, 3, '4444', 'dddduuuddduddd', 1, 1, '0', 317, 3642, 63, 'THT', 'False'),
(2, 2, 'faaIdllladawddEIffu', 'pppppppppppp', 1, 127, 'p22', 4744, NULL, NULL, 'z5zz5', 'Scunthorpe'),
(3, 1, 'ZyZyhZh9y', 'yy', 1, 31, '5', 82, NULL, 749, 'none', NULL);
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
(1, 1, 3, 1, '(voice)', 15, 2),
(2, 2, 2, 1, '(uncredited)', 1002, 2);
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
(2, 2, 1, 2),
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
(1, 3, 1, 1, 'Inf');
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
(1, 1, 2, 'Bulgaria', 'NUL');
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
(1, 1, 1, '4.0', ''),
(2, 2, 2, '8.0', 'True');
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
(2, 3, 3),
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
(1, 2, 3, 1),
(2, 2, 3, 2);
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
(1, 2, 1, '8nDs', '__dict__'),
(2, 2, 1, 'LdLLXXLXLdLXXnXccX', 'Y');
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
(1, 'True', '[us]', NULL, 'IIIIIhI', 'false', NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'GG'),
(2, 'hero-sequel', 'hhhhh0G'),
(3, 'marvel-cinematic-universe', 'FooF');
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
(1, 'Downey Jr., Robert', 'vMvMrErrvM', 63, '9j9jjjjb9ojboo', '4', 'riErE', 'NIL', 'tDi'),
(2, 'Downey Jr., Robert', 'false', 4096, 'then', 'Scunthorpe', 'True', 'TRUE', 'then');
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
(1, 'Voice Character', NULL, 2666, 'aa5a5aaaa55a555a5a5', NULL, NULL),
(2, 'Voice Character', '', 494, 'ZZZ', '5wgBWBSW', '0'),
(3, 'Other Character', '', 98, '', 'M', '1e100');
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
(1, '__proto__', '1Yt', 1, 2012, 1102, 'LPT1', 128, 255, 127, '1e100', 'Inf'),
(2, 'JJJJJJ', 'sX_ic', 1, 2007, 16, 'TRUE', 16, 1, 6, 'zt', 'V'),
(3, 'undefined', '%%y%', 1, 2006, 255, 'nil', 2, 717, 3565, 'False', '0SCy0CyUCUiyCU');
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
(1, 2, 'cY', 'iiii', '', NULL, NULL, 'Scunthorpe'),
(2, 2, '00', NULL, '', NULL, NULL, 'None');
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
(1, 2, 'C_', 'hbhbhhhhhhbh', 1, NULL, NULL, NULL, NULL, NULL, 'dddduuuddduddd', NULL),
(2, 2, '1', NULL, 1, 317, '1', NULL, 1, NULL, 'False', NULL),
(3, 2, '1', NULL, 1, NULL, '1', NULL, NULL, 4744, NULL, NULL);
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
(2, 2, 2, 2, NULL, NULL, 2);
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
(1, 2, 1, '4.0', NULL),
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
(1, 2, 2),
(2, 2, 2),
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
(1, 3, 2, 2),
(2, 1, 2, 2);
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
(2, 1, 2, 'Inf', NULL);
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
(1, 'True', '[us]', NULL, 'IIIIIhI', 'false', NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'GG'),
(2, 'hero-sequel', 'hhhhh0G'),
(3, 'marvel-cinematic-universe', 'FooF');
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
(1, 'Downey Jr., Robert', 'vMvMrErrvM', 63, '9j9jjjjb9ojboo', '4', 'riErE', 'NIL', 'tDi'),
(2, 'Downey Jr., Robert', 'false', 4096, 'then', 'Scunthorpe', 'True', 'TRUE', 'then');
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
(1, 'Voice Character', NULL, 2666, 'aa5a5aaaa55a555a5a5', NULL, NULL),
(2, 'Voice Character', '', 494, 'ZZZ', '5wgBWBSW', '0'),
(3, 'Other Character', '', 98, '', 'M', '1e100');
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
(1, '__proto__', '1Yt', 1, 2012, 1102, 'LPT1', 128, 255, 127, '1e100', 'Inf'),
(2, 'JJJJJJ', 'sX_ic', 1, 2006, 16, 'TRUE', 16, 1, 6, 'zt', 'V'),
(3, 'undefined', '%%y%', 1, 2006, 255, 'nil', 2, 717, 3565, 'False', '0SCy0CyUCUiyCU');
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
(1, 2, 'cY', 'iiii', NULL, 'XuXllulluXX', 'Scunthorpe', '00'),
(2, 1, '', 'SpDpdpPh4pdaD', 'None', 'C_', 'hbhbhhhhhhbh', '1');
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
(1, 3, '4444', 'dddduuuddduddd', 1, 1, '0', 317, 3642, 63, 'THT', 'False'),
(2, 2, 'faaIdllladawddEIffu', 'pppppppppppp', 1, 127, 'p22', 4744, NULL, NULL, 'z5zz5', 'Scunthorpe'),
(3, 1, 'ZyZyhZh9y', 'yy', 1, 31, '5', 82, NULL, 749, 'none', NULL);
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
(1, 1, 3, 1, '(voice)', 15, 2),
(2, 2, 2, 1, '(uncredited)', 1002, 2);
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
(2, 2, 1, 2),
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
(1, 3, 1, 1, 'Inf');
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
(1, 1, 2, 'Bulgaria', 'NUL');
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
(1, 1, 1, '4.0', ''),
(2, 2, 2, '8.0', 'True');
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
(2, 3, 3),
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
(1, 2, 3, 1),
(2, 2, 3, 2);
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
(1, 2, 1, '8nDs', '__dict__'),
(2, 2, 1, 'LdLLXXLXLdLXXnXccX', 'Y');
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
(1, 'True', '[us]', NULL, 'IIIIIhI', 'false', NULL);
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'GG'),
(2, 'hero-sequel', 'hhhhh0G'),
(3, 'marvel-cinematic-universe', 'FooF');
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
(1, 'Downey Jr., Robert', NULL, NULL, NULL, '1', NULL, NULL, '4'),
(2, 'Other Person', NULL, NULL, 'NIL', 'tDi', NULL, 'false', '1');
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
(1, 'Other Character', NULL, 1, NULL, 'TRUE', 'then'),
(2, 'Other Character', NULL, NULL, NULL, NULL, '1'),
(3, 'Other Character', NULL, NULL, NULL, NULL, '');
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
(1, '1', NULL, 1, NULL, NULL, '5wgBWBSW', 1, NULL, NULL, '', '1'),
(2, '1', 'M', 1, NULL, NULL, NULL, 1, NULL, NULL, '1', NULL),
(3, '1', NULL, 1, 2006, 255, '1', NULL, NULL, 1, NULL, NULL);
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
(1, 2, 'sX_ic', NULL, '1', NULL, 'TRUE', '1'),
(2, 1, '', NULL, 'zt', 'V', NULL, '%%y%');
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
(1, 1, '', NULL, 1, NULL, 'nil', 2, 717, 3565, 'False', '0SCy0CyUCUiyCU'),
(2, 2, 'cY', 'iiii', 1, 1, NULL, 1, NULL, 1, NULL, NULL),
(3, 2, '', NULL, 1, 1, NULL, 1, NULL, 1, NULL, '1');
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
(2, 2, 1, 2, NULL, 317, 2);
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
(1, 2, 2, '4.0', '1'),
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
(1, 2, 2),
(2, 2, 1),
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
(1, 2, 2, 2),
(2, 1, 2, 2);
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
-- Generated database 076/100
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
(1, 'Ic', '[ru]', NULL, 'PNPPxNPNNN', 'Vo434IHIVuNFF4oN4oINII', '33'),
(2, 'S', NULL, 344, 'DggDDl3tgl3B', '__dict__', 'Ot9WtXtvv9'),
(3, 'bb', '[ru]', 322, 'oPO', 'zMMxzMB', 'HGGHHHHHG');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', '__proto__'),
(3, 'hero-sequel', 'W''i''''''iWWi');
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
(1, 'Other Person', '', 1678, '-Infinity', NULL, 'undefined', 'M%MM%', '2'),
(2, 'Other Person', 'kkk1k1k', 5667, NULL, 'Scunthorpe', 'nil', 'WO', 'qvqsVsVT');
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
(1, 'Other Character', '7TTxT', 8432, 'iSi', 'else', 'mmQiDmdd'),
(2, 'Other Character', 'iiihhhh', 346, '77jjj', 'QNQ', '');
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
(1, 'iiOiiy33sOO3y3', 'J', 1, 2010, 187, 'FALSE', 9352, 5998, 2273, NULL, 'T'),
(2, 'Inf', 'WN', 1, 2012, 2048, 'V1Hb1C1', NULL, 0, 1053, 'NUL', 'ttt'),
(3, '6', '00', 1, 2005, 1024, '000D005H5D0HHH55H055505', 403, 8191, 583, 'B4k', 'NIL');
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
(1, 1, 'DEEUDDUmzzD5c55U', NULL, 'false', '999999999999999999999999999999', 'BY4Yy4yB44', 'INF');
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
(1, 3, 'b', 'y', 1, NULL, 'FALSE', 8, 8, 127, 'CyI-', ''),
(2, 1, 'q', NULL, 1, 4, 'eGn', 256, 511, NULL, 'none', 'VM_'),
(3, 1, 'WHW', NULL, 1, 3910, NULL, 4151, NULL, 3841, 'Ug', '');
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
(1, 1, 3, 2, '(uncredited)', 8191, 1),
(2, 2, 1, 2, NULL, NULL, 2),
(3, 1, 2, 1, '(uncredited)', 63, 1);
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
(2, 1, 2, 1),
(3, 2, 2, 1);
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
(1, 2, 2, 3, 'bWEb');
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
(1, 3, 1, 'Bulgaria', 'etvltteBeO%est%BVlOOs'),
(2, 1, 1, 'Bulgaria', 'BNCNNN'),
(3, 2, 2, 'USA', 'cc0c080ojoYjo');
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
(1, 1, 2, '8.0', 'true'),
(2, 3, 2, '4.0', 'wwwwwwww');
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
(1, 1, 2, '', 'vvv1'),
(2, 1, 2, 'NIL', 'False'),
(3, 1, 2, 'BtBB', 'HyRHyHF');
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
(1, 'Ic', '[ru]', NULL, 'PNPPxNPNNN', 'Vo434IHIVuNFF4oN4oINII', '33'),
(2, 'S', NULL, 344, 'DggDDl3tgl3B', '__dict__', 'Ot9WtXtvv9'),
(3, 'bb', '[ru]', 322, 'oPO', 'zMMxzMB', 'HGGHHHHHG');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', '__proto__'),
(3, 'hero-sequel', 'W''i''''''iWWi');
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
(1, 'Other Person', '', 1678, '-Infinity', NULL, 'undefined', 'M%MM%', '2'),
(2, 'Other Person', 'kkk1k1k', 5667, NULL, 'Scunthorpe', 'nil', 'WO', 'qvqsVsVT');
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
(1, 'Other Character', '7TTxT', 8432, 'iSi', 'else', 'mmQiDmdd'),
(2, 'Other Character', 'iiihhhh', 346, '77jjj', 'QNQ', '');
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
(1, 'iiOiiy33sOO3y3', 'J', 1, 2010, 187, 'FALSE', 9352, 5998, 2273, NULL, 'T'),
(2, 'Inf', 'WN', 1, 2012, 2048, 'V1Hb1C1', NULL, 0, 1053, 'NUL', 'ttt'),
(3, '6', '00', 1, 2005, 1024, '000D005H5D0HHH55H055505', 403, 8191, 583, 'B4k', 'NIL');
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
(1, 1, 'DEEUDDUmzzD5c55U', NULL, 'false', '999999999999999999999999999999', 'BY4Yy4yB44', 'INF');
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
(1, 3, 'b', 'y', 1, NULL, 'FALSE', 8, 8, 127, 'CyI-', ''),
(2, 1, 'q', NULL, 1, 4, 'eGn', 256, 511, NULL, 'none', 'VM_'),
(3, 1, 'WHW', NULL, 1, 3910, NULL, 4151, NULL, 3841, 'Ug', '');
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
(1, 1, 3, 2, '(uncredited)', 8191, 1),
(2, 2, 1, 2, NULL, NULL, 2),
(3, 1, 2, 1, '(uncredited)', 63, 1);
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
(2, 1, 2, 1),
(3, 2, 1, 1);
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
(1, 2, 2, 3, 'bWEb');
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
(1, 3, 1, 'Bulgaria', 'etvltteBeO%est%BVlOOs'),
(2, 1, 1, 'Bulgaria', 'BNCNNN'),
(3, 2, 2, 'USA', 'cc0c080ojoYjo');
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
(1, 1, 2, '8.0', 'true'),
(2, 3, 2, '4.0', 'wwwwwwww');
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
(1, 1, 2, '', 'vvv1'),
(2, 1, 2, 'NIL', 'False'),
(3, 1, 2, 'BtBB', 'HyRHyHF');
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
(1, 'Ic', '[ru]', NULL, 'PNPPxNPNNN', 'Vo434IHIVuNFF4oN4oINII', '33'),
(2, 'S', NULL, 344, 'DggDDl3tgl3B', '__dict__', 'Ot9WtXtvv9'),
(3, 'bb', '[ru]', 322, 'oPO', 'zMMxzMB', 'HGGHHHHHG');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', '__proto__'),
(3, 'hero-sequel', 'W''i''''''iWWi');
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
(1, 'Other Person', '', 1678, '-Infinity', NULL, 'undefined', 'M%MM%', '2'),
(2, 'Other Person', 'kkk1k1k', 5667, NULL, 'Scunthorpe', 'nil', 'WO', 'qvqsVsVT');
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
(1, 'Other Character', '7TTxT', 8432, 'iSi', 'else', 'mmQiDmdd'),
(2, 'Other Character', 'iiihhhh', 1, '77jjj', 'QNQ', '');
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
(1, 'iiOiiy33sOO3y3', 'J', 1, 2010, 187, 'FALSE', 9352, 5998, 2273, NULL, 'T'),
(2, 'Inf', 'WN', 1, 2012, 2048, 'V1Hb1C1', NULL, 0, 1053, 'NUL', 'ttt'),
(3, '6', '00', 1, 2005, 1024, '000D005H5D0HHH55H055505', 403, 8191, 583, 'B4k', 'NIL');
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
(1, 1, 'DEEUDDUmzzD5c55U', NULL, 'false', '999999999999999999999999999999', 'BY4Yy4yB44', 'INF');
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
(1, 3, 'b', 'y', 1, NULL, 'FALSE', 8, 8, 127, 'CyI-', ''),
(2, 1, 'q', NULL, 1, 4, 'eGn', 256, 511, NULL, 'none', 'VM_'),
(3, 1, 'WHW', NULL, 1, 3910, NULL, 4151, NULL, 3841, 'Ug', '');
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
(1, 1, 3, 2, '(uncredited)', 8191, 1),
(2, 2, 1, 2, NULL, NULL, 2),
(3, 1, 2, 1, '(uncredited)', 63, 1);
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
(2, 1, 2, 1),
(3, 2, 2, 1);
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
(1, 2, 2, 3, 'bWEb');
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
(1, 3, 1, 'Bulgaria', 'etvltteBeO%est%BVlOOs'),
(2, 1, 1, 'Bulgaria', 'BNCNNN'),
(3, 2, 2, 'USA', 'cc0c080ojoYjo');
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
(1, 1, 2, '8.0', 'true'),
(2, 3, 2, '4.0', 'wwwwwwww');
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
(1, 1, 2, '', 'vvv1'),
(2, 1, 2, 'NIL', 'False'),
(3, 1, 2, 'BtBB', 'HyRHyHF');
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
(1, 'Ic', '[ru]', NULL, 'PNPPxNPNNN', 'Vo434IHIVuNFF4oN4oINII', '33'),
(2, 'S', NULL, 344, 'DggDDl3tgl3B', '__dict__', 'Ot9WtXtvv9'),
(3, 'bb', '[ru]', 322, 'oPO', 'zMMxzMB', 'HGGHHHHHG');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', '__proto__'),
(3, 'hero-sequel', 'W''i''''''iWWi');
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
(1, 'Other Person', '', 1678, '-Infinity', NULL, 'undefined', 'M%MM%', '2'),
(2, 'Other Person', 'kkk1k1k', 5667, NULL, 'Scunthorpe', 'nil', 'WO', 'qvqsVsVT');
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
(1, 'Other Character', '7TTxT', 8432, 'iSi', 'else', 'mmQiDmdd'),
(2, 'Other Character', 'iiihhhh', 346, '77jjj', 'QNQ', '');
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
(1, 'iiOiiy33sOO3y3', 'J', 1, 2010, 187, 'FALSE', 9352, 5998, 2273, NULL, 'T'),
(2, 'Inf', 'WN', 1, 2012, 2048, 'V1Hb1C1', NULL, 0, 1053, 'NUL', 'ttt'),
(3, '6', '00', 1, 2005, 1024, '000D005H5D0HHH55H055505', 403, 8191, 583, 'B4k', 'NIL');
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
(1, 1, 'DEEUDDUmzzD5c55U', NULL, 'false', '999999999999999999999999999999', 'BY4Yy4yB44', 'INF');
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
(1, 3, 'b', 'y', 1, NULL, 'FALSE', 8, 8, 127, 'CyI-', ''),
(2, 1, 'q', NULL, 1, 4, 'eGn', 256, 511, NULL, 'none', 'VM_'),
(3, 1, 'WHW', NULL, 1, 3910, NULL, 4151, NULL, 3841, 'Ug', '');
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
(1, 1, 3, 2, '(uncredited)', 8191, 1),
(2, 2, 1, 2, NULL, NULL, 2),
(3, 1, 2, 1, '(uncredited)', 63, 1);
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
(2, 1, 2, 1),
(3, 2, 2, 1);
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
(1, 2, 2, 3, 'bWEb');
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
(1, 3, 1, 'Bulgaria', 'etvltteBeO%est%BVlOOs'),
(2, 1, 1, 'Bulgaria', 'BNCNNN'),
(3, 2, 2, 'USA', 'cc0c080ojoYjo');
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
(1, 1, 2, '8.0', 'true'),
(2, 3, 2, '4.0', 'wwwwwwww');
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
(1, 1, 2, '', 'vvv1'),
(2, 1, 1, 'NIL', 'False'),
(3, 1, 2, 'BtBB', 'HyRHyHF');
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
(1, 'Ic', '[ru]', NULL, 'PNPPxNPNNN', 'NIL', '33'),
(2, 'S', NULL, 344, 'DggDDl3tgl3B', '__dict__', 'Ot9WtXtvv9'),
(3, 'bb', '[ru]', 322, 'oPO', 'zMMxzMB', 'HGGHHHHHG');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
(2, 'marvel-cinematic-universe', '__proto__'),
(3, 'hero-sequel', 'W''i''''''iWWi');
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
(1, 'Other Person', '', 1678, '-Infinity', NULL, 'undefined', 'M%MM%', '2'),
(2, 'Other Person', 'kkk1k1k', 5667, NULL, 'Scunthorpe', 'nil', 'WO', 'qvqsVsVT');
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
(1, 'Other Character', '7TTxT', 8432, 'iSi', 'else', 'mmQiDmdd'),
(2, 'Other Character', 'iiihhhh', 346, '77jjj', 'QNQ', '');
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
(1, 'iiOiiy33sOO3y3', 'J', 1, 2010, 187, 'FALSE', 9352, 5998, 2273, NULL, 'T'),
(2, 'Inf', 'WN', 1, 2012, 2048, 'V1Hb1C1', NULL, 0, 1053, 'NUL', 'ttt'),
(3, '6', '00', 1, 2005, 1024, '000D005H5D0HHH55H055505', 403, 8191, 583, 'B4k', 'NIL');
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
(1, 1, 'DEEUDDUmzzD5c55U', NULL, 'false', '999999999999999999999999999999', 'BY4Yy4yB44', 'INF');
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
(1, 3, 'b', 'y', 1, NULL, 'FALSE', 8, 8, 127, 'CyI-', ''),
(2, 1, 'q', NULL, 1, 4, 'eGn', 256, 511, NULL, 'none', 'VM_'),
(3, 1, 'WHW', NULL, 1, 3910, NULL, 4151, NULL, 3841, 'Ug', '');
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
(1, 1, 3, 2, '(uncredited)', 8191, 1),
(2, 2, 1, 2, NULL, NULL, 2),
(3, 1, 2, 1, '(uncredited)', 63, 1);
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
(2, 1, 2, 1),
(3, 2, 2, 1);
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
(1, 2, 2, 3, 'bWEb');
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
(1, 3, 1, 'Bulgaria', 'etvltteBeO%est%BVlOOs'),
(2, 1, 1, 'Bulgaria', 'BNCNNN'),
(3, 2, 2, 'USA', 'cc0c080ojoYjo');
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
(1, 1, 2, '8.0', 'true'),
(2, 3, 2, '4.0', 'wwwwwwww');
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
(1, 1, 2, '', 'vvv1'),
(2, 1, 2, 'NIL', 'False'),
(3, 1, 2, 'BtBB', 'HyRHyHF');
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
(1, 'o', '[us]', 4095, 'GDIID_zI', '00', ''),
(2, 'Lo-oL--vL-QQ', '[us]', NULL, 'A', 'wCChChhwCCChwwwhCChCwwChChwwhhhC', 'FF-F-');
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
(1, 'marvel-cinematic-universe', '_rrrruGXG02X_'),
(2, 'character-name-in-title', 'None'),
(3, 'hero-sequel', 'J');
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
(1, 'Downey Jr., Robert', 'PQP', 511, 'mhhhgggYggYhhhhmhhYgY', 'j xExjMNx  EjN', 'NaN', 'null', 'bLliLWia');
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
(1, 'Voice Character', 'J', 4095, 'HHm', '', NULL);
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
(1, 'LPT1', 'ghvJvJc', 3, NULL, 8374, '2Sb_222_2_22_SS_SSSbSS2', 343, 691, NULL, 'r80rQ008r8QQ80r00', 'Rsbms37mbPb3m7b'),
(2, 'NULL', NULL, 1, 2005, NULL, 'wiwwwwwwjiiwjwwiijw', 15, 512, 8, 'H', 'ttI');
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
(1, 1, 'wwwwwwwwww', NULL, NULL, 'lMq''', 'FALSE', '44SANS'),
(2, 1, 'L', NULL, NULL, 'ooooi', 'none', NULL);
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
(1, 2, 'q5zxzxqx5z-qq', '', 2, 395, ' ', NULL, 63, 8, 'j%%%j''DQ%', 'OssOO'),
(2, 2, 'TKF', '7799', 3, 255, 'TQT', 399, 64, 2, '-Infinity', '0');
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
(1, 1, 2, 1, '(voice)', 4096, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 1, 1),
(3, 1, 1, 1, '(voice) (uncredited)', 63, 2);
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
(1, 1, 1, 2);
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
(1, 2, 1, 1, '1e100'),
(2, 2, 1, 2, 'tV');
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
(1, 2, 1, 'Bulgaria', 'nil'),
(2, 1, 1, 'USA', 'True'),
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
(1, 2, 1, '4.0', ''),
(2, 2, 1, '4.0', 'NULL'),
(3, 2, 1, '4.0', '9');
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
(1, 1, 2, 3);
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
(1, 1, 1, 'True', 'NULL'),
(2, 1, 1, 'JcVJ_AVJAA1c_A', 'XJX');
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
(1, 'o', '[us]', 4095, 'GDIID_zI', '00', ''),
(2, 'Lo-oL--vL-QQ', '[us]', NULL, 'A', 'wCChChhwCCChwwwhCChCwwChChwwhhhC', 'FF-F-');
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
(1, 'marvel-cinematic-universe', '_rrrruGXG02X_'),
(2, 'character-name-in-title', 'None'),
(3, 'hero-sequel', 'J');
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
(1, 'Downey Jr., Robert', 'PQP', 511, 'mhhhgggYggYhhhhmhhYgY', 'j xExjMNx  EjN', 'NaN', 'null', 'bLliLWia');
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
(1, 'Voice Character', 'J', 4095, 'HHm', '', NULL);
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
(1, 'LPT1', 'ghvJvJc', 3, NULL, 8374, '2Sb_222_2_22_SS_SSSbSS2', 343, 691, NULL, 'r80rQ008r8QQ80r00', 'Rsbms37mbPb3m7b'),
(2, 'NULL', NULL, 1, 2005, NULL, 'wiwwwwwwjiiwjwwiijw', 15, 512, 8, 'H', 'ttI');
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
(1, 1, 'wwwwwwwwww', NULL, NULL, 'lMq''', 'FALSE', '44SANS'),
(2, 1, 'L', NULL, NULL, 'ooooi', 'none', NULL);
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
(1, 2, 'q5zxzxqx5z-qq', '', 2, 395, ' ', NULL, 63, 8, 'j%%%j''DQ%', 'OssOO'),
(2, 2, 'TKF', '7799', 3, 255, 'TQT', 399, 64, 2, '-Infinity', '0');
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
(1, 1, 2, 1, '(voice)', 4096, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 1, 1),
(3, 1, 1, 1, '(voice) (uncredited)', 63, 2);
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
(1, 1, 1, 2);
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
(1, 2, 1, 1, '1e100'),
(2, 2, 1, 2, 'tV');
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
(1, 2, 1, 'Bulgaria', 'nil'),
(2, 1, 1, 'USA', NULL),
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
(1, 2, 1, '4.0', NULL),
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
(1, 2, 2),
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
(1, 1, 1, '1', NULL),
(2, 1, 1, '1', NULL);
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
(1, 'o', '[us]', 4095, 'GDIID_zI', '00', ''),
(2, 'Lo-oL--vL-QQ', '[us]', NULL, 'A', 'wCChChhwCCChwwwhCChCwwChChwwhhhC', 'FF-F-');
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
(1, 'marvel-cinematic-universe', '_rrrruGXG02X_'),
(2, 'character-name-in-title', 'None'),
(3, 'hero-sequel', 'J');
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
(1, 'Downey Jr., Robert', 'PQP', 511, 'mhhhgggYggYhhhhmhhYgY', 'j xExjMNx  EjN', 'NaN', 'null', 'bLliLWia');
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
(1, 'Voice Character', 'J', 4095, 'HHm', '', NULL);
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
(1, 'LPT1', 'ghvJvJc', 3, NULL, 8374, '2Sb_222_2_22_SS_SSSbSS2', 343, 691, NULL, 'r80rQ008r8QQ80r00', 'Rsbms37mbPb3m7b'),
(2, 'NULL', NULL, 1, 2005, NULL, 'wiwwwwwwjiiwjwwiijw', 15, 512, 8, 'H', 'ttI');
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
(1, 1, 'wwwwwwwwww', NULL, NULL, 'lMq''', 'FALSE', '44SANS'),
(2, 1, 'L', NULL, NULL, 'ooooi', 'none', NULL);
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
(1, 2, 'q5zxzxqx5z-qq', '', 2, 395, ' ', NULL, 63, 8, '', NULL),
(2, 2, 'OssOO', NULL, 2, NULL, '7799', NULL, 255, 1, NULL, '1');
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
(1, 1, 2, NULL, '(voice)', NULL, 2),
(2, 1, 2, NULL, NULL, 1, 2),
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
(1, 1, 2, 2);
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
(2, 1, 2, 1, '1');
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
(1, 2, 1, '4.0', 'tV'),
(2, 2, 1, '8.0', 'nil'),
(3, 1, 1, '4.0', 'True');
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
(1, 1, 1, '1', NULL),
(2, 1, 1, '1', NULL);
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
(1, 'o', '[us]', 4095, 'GDIID_zI', '00', ''),
(2, 'Lo-oL--vL-QQ', '[us]', NULL, 'A', 'wCChChhwCCChwwwhCChCwwChChwwhhhC', 'FF-F-');
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
(1, 'marvel-cinematic-universe', '_rrrruGXG02X_'),
(2, 'character-name-in-title', 'None'),
(3, 'hero-sequel', 'J');
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
(1, 'Downey Jr., Robert', 'PQP', 511, 'mhhhgggYggYhhhhmhhYgY', 'j xExjMNx  EjN', 'NaN', 'null', 'bLliLWia');
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
(1, 'Voice Character', 'J', 4095, 'HHm', '', NULL);
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
(1, 'LPT1', 'ghvJvJc', 3, NULL, 8374, '2Sb_222_2_22_SS_SSSbSS2', 343, 691, NULL, 'r80rQ008r8QQ80r00', 'Rsbms37mbPb3m7b'),
(2, 'NULL', NULL, 1, 2005, NULL, 'wiwwwwwwjiiwjwwiijw', 15, 512, 8, 'H', 'ttI');
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
(1, 1, 'wwwwwwwwww', NULL, NULL, 'lMq''', 'FALSE', '44SANS'),
(2, 1, 'L', NULL, NULL, 'ooooi', 'none', NULL);
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
(1, 2, 'q5zxzxqx5z-qq', '', 2, 395, ' ', NULL, 63, 8, 'j%%%j''DQ%', 'OssOO'),
(2, 2, 'TKF', '7799', 3, 255, 'TQT', 399, 64, 2, '-Infinity', '0');
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
(1, 1, 2, 1, '(voice)', 4096, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 1, 1),
(3, 1, 1, 1, '(voice) (uncredited)', 63, 2);
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
(1, 1, 1, 2);
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
(1, 2, 1, 1, '1e100'),
(2, 2, 1, 2, 'tV');
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
(1, 2, 1, 'USA', 'nil'),
(2, 1, 1, 'USA', 'True'),
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
(1, 2, 1, '4.0', ''),
(2, 2, 1, '4.0', 'NULL'),
(3, 2, 1, '4.0', '9');
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
(1, 1, 2, 3);
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
(1, 1, 1, 'True', 'NULL'),
(2, 1, 1, 'JcVJ_AVJAA1c_A', 'XJX');
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
(1, 'o', '[us]', 4095, 'GDIID_zI', '00', ''),
(2, 'Lo-oL--vL-QQ', '[us]', NULL, 'A', 'wCChChhwCCChwwwhCChCwwChChwwhhhC', 'FF-F-');
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
(1, 'marvel-cinematic-universe', '_rrrruGXG02X_'),
(2, 'character-name-in-title', 'None'),
(3, 'hero-sequel', 'J');
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
(1, 'Downey Jr., Robert', 'PQP', 511, 'mhhhgggYggYhhhhmhhYgY', 'j xExjMNx  EjN', 'NaN', 'null', 'bLliLWia');
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
(1, 'Voice Character', 'J', 4095, 'HHm', '', NULL);
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
(1, 'LPT1', 'ghvJvJc', 3, NULL, 8374, '2Sb_222_2_22_SS_SSSbSS2', 343, 691, NULL, 'r80rQ008r8QQ80r00', 'Rsbms37mbPb3m7b'),
(2, 'NULL', NULL, 1, 2005, NULL, 'wiwwwwwwjiiwjwwiijw', 15, 512, 8, 'H', 'ttI');
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
(1, 1, 'Rsbms37mbPb3m7b', NULL, NULL, 'lMq''', 'FALSE', '44SANS'),
(2, 1, 'L', NULL, NULL, 'ooooi', 'none', NULL);
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
(1, 2, 'q5zxzxqx5z-qq', '', 2, 395, ' ', NULL, 63, 8, 'j%%%j''DQ%', 'OssOO'),
(2, 2, 'TKF', '7799', 3, 255, 'TQT', 399, 64, 2, '-Infinity', '0');
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
(1, 1, 2, 1, '(voice)', 4096, 1),
(2, 1, 1, 1, '(voice) (uncredited)', 1, 1),
(3, 1, 1, 1, '(voice) (uncredited)', 63, 2);
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
(1, 1, 1, 2);
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
(1, 2, 1, 1, '1e100'),
(2, 2, 1, 2, 'tV');
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
(1, 2, 1, 'Bulgaria', 'nil'),
(2, 1, 1, 'USA', 'True'),
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
(1, 2, 1, '4.0', ''),
(2, 2, 1, '4.0', 'NULL'),
(3, 2, 1, '4.0', '9');
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
(1, 1, 2, 3);
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
(1, 1, 1, 'True', 'NULL'),
(2, 1, 1, 'JcVJ_AVJAA1c_A', 'XJX');
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
(1, '', '[ru]', 275, 'True', ' gTPPR0TP3T3 P', 'r-UN');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '5OOfVfss51O'),
(2, 'character-name-in-title', 'S'),
(3, 'marvel-cinematic-universe', 'p44Wp4Wp');
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
(1, 'Other Person', NULL, 3883, '', NULL, '_v_a4_4_ia', 'nil', 'TRUE'),
(2, 'Other Person', 'ffk', 159, 'K', 'None', 'IIIIIIIIIIIIIIIIIII', '', 'sGfG');
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
(1, 'Voice Character', '_H', 410, 'M39aMM', 'a3', ''),
(2, 'Voice Character', '7q7qZff77q', 642, 'ttt QfdtQf ', '', '0');
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
(1, 'YYxoxYxYvYYvvvvYxov', '0', 1, NULL, 1055, '%%CC77o77%%7', 696, 36, 8434, NULL, 'none'),
(2, 'kk', 'SEwSE', 1, 2012, 6242, 'DMzzDz2', 332, 4322, 523, 'NaN', 'Infinity');
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
(1, 2, 'INF', 'Inf', 'AM', '__proto__', NULL, 'UkUUIUIID'),
(2, 2, '   ''', '', 'ddddddddd', 'w9CCk99nC', NULL, '0');
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
(1, 2, 'YT5YT5665T5TT', '', 2, 1973, 'undefined', 1659, 4624, 0, 'INF', 'Pj');
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
(1, 2, 1, 1, '(uncredited)', 1362, 1),
(2, 2, 2, 2, '(voice) (uncredited)', 6275, 1);
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
(1, 1, 1, 2, 'D');
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
(1, 1, 2, 'USA', 'BBr'),
(2, 2, 2, 'USA', 'm');
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
(1, 1, 3),
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
(1, 2, 2, 3),
(2, 1, 2, 2);
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
(1, 2, 2, 'LLLLtttLtLLtttttLLtt', '6INII');
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
(1, '', '[ru]', 275, 'True', ' gTPPR0TP3T3 P', 'r-UN');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '5OOfVfss51O'),
(2, 'character-name-in-title', 'S'),
(3, 'marvel-cinematic-universe', 'p44Wp4Wp');
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
(1, 'Other Person', NULL, 3883, '', NULL, '_v_a4_4_ia', 'nil', 'TRUE'),
(2, 'Other Person', 'ffk', 159, 'K', 'None', 'IIIIIIIIIIIIIIIIIII', '', 'sGfG');
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
(1, 'Voice Character', '_H', 410, 'M39aMM', 'a3', ''),
(2, 'Voice Character', '7q7qZff77q', 642, 'ttt QfdtQf ', '', '0');
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
(1, 'YYxoxYxYvYYvvvvYxov', '0', 1, NULL, 1055, '%%CC77o77%%7', 696, 36, 8434, NULL, 'none'),
(2, 'kk', 'SEwSE', 1, 2012, 6242, 'DMzzDz2', 332, 4322, 523, 'NaN', 'Infinity');
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
(1, 2, 'INF', 'Inf', 'AM', '__proto__', NULL, 'UkUUIUIID'),
(2, 2, '   ''', '', 'ddddddddd', 'w9CCk99nC', NULL, '0');
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
(1, 2, 'YT5YT5665T5TT', '', 2, 1973, 'undefined', 4322, 4624, 0, 'INF', 'Pj');
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
(1, 2, 1, 1, '(uncredited)', 1362, 1),
(2, 2, 2, 2, '(voice) (uncredited)', 6275, 1);
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
(1, 1, 1, 2, 'D');
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
(1, 1, 2, 'USA', 'BBr'),
(2, 2, 2, 'USA', 'm');
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
(1, 1, 3),
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
(1, 2, 2, 3),
(2, 1, 2, 2);
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
(1, 2, 2, 'LLLLtttLtLLtttttLLtt', '6INII');
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
(1, '', '[ru]', 275, 'True', ' gTPPR0TP3T3 P', 'r-UN');
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
(2, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', NULL),
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
(1, 'Other Person', NULL, NULL, NULL, NULL, NULL, NULL, NULL),
(2, 'Other Person', '1', 1, '_v_a4_4_ia', 'nil', 'TRUE', NULL, 'ffk');
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
(1, 'Other Character', 'K', 1, NULL, 'IIIIIIIIIIIIIIIIIII', ''),
(2, 'Other Character', NULL, NULL, NULL, NULL, '_H');
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
(1, '1', NULL, 2, NULL, NULL, 'a3', 1, NULL, 1, NULL, '1'),
(2, '1', NULL, 2, 2006, NULL, NULL, NULL, 1, NULL, NULL, NULL);
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
(1, 2, '1', NULL, NULL, '1', NULL, '1'),
(2, 2, 'none', NULL, 'SEwSE', NULL, '1', NULL);
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
(1, 2, 'DMzzDz2', '1', 2, 523, 'NaN', 1, NULL, NULL, NULL, NULL);
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
(2, 2, 2, NULL, '(voice)', NULL, 1);
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
(1, 2, 2, '4.0', NULL);
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
(1, 2, 2, 2),
(2, 2, 2, 2);
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
(1, 2, 2, '1', '1');
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
(1, '', '[ru]', 275, 'True', ' gTPPR0TP3T3 P', 'r-UN');
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
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', '5OOfVfss51O'),
(2, 'character-name-in-title', 'S'),
(3, 'marvel-cinematic-universe', 'p44Wp4Wp');
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
(1, 'Other Person', NULL, 3883, '', NULL, '_v_a4_4_ia', 'nil', 'TRUE'),
(2, 'Other Person', 'ffk', 159, 'K', 'None', 'IIIIIIIIIIIIIIIIIII', '', 'sGfG');
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
(1, 'Voice Character', '_H', 410, 'M39aMM', 'a3', ''),
(2, 'Voice Character', '7q7qZff77q', 642, 'ttt QfdtQf ', '', '0');
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
(1, 'YYxoxYxYvYYvvvvYxov', '0', 1, NULL, 1055, '%%CC77o77%%7', 696, 2, 8434, NULL, 'none'),
(2, 'kk', 'SEwSE', 1, 2012, 6242, 'DMzzDz2', 332, 4322, 523, 'NaN', 'Infinity');
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
(1, 2, 'INF', 'Inf', 'AM', '__proto__', NULL, 'UkUUIUIID'),
(2, 2, '   ''', '', 'ddddddddd', 'w9CCk99nC', NULL, '0');
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
(1, 2, 'YT5YT5665T5TT', '', 2, 1973, 'undefined', 1659, 4624, 0, 'INF', 'Pj');
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
(1, 2, 1, 1, '(uncredited)', 1362, 1),
(2, 2, 2, 2, '(voice) (uncredited)', 6275, 1);
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
(1, 1, 1, 2, 'D');
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
(1, 1, 2, 'USA', 'BBr'),
(2, 2, 2, 'USA', 'm');
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
(1, 1, 3),
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
(1, 2, 2, 3),
(2, 1, 2, 2);
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
(1, 2, 2, 'LLLLtttLtLLtttttLLtt', '6INII');
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
(1, '', '[ru]', 1165, 'O''n', 'aq_q88Ya_N', NULL),
(2, 'hc', '[ru]', 631, 'NIL', '-Infinity', 'nil');
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
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'True'),
(2, 'hero-sequel', 'NtWNWW'),
(3, 'character-name-in-title', 'LPT1');
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
(1, 'Downey Jr., Robert', NULL, 276, '7wFw', 'FuguREFg__8', 'CjCLjCCLCejkeCzz''eb''ejCjCCC', NULL, 'b');
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
(1, 'Other Character', '0', NULL, 'uGsDssGGD1', 'MgMggM', 'LPT1'),
(2, 'Other Character', 's', 9575, 'pmpY_mppmmY__m_mm_mT_pTYTmTYYmm_', '_ww_wwww', '');
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
(1, 'MMbbtTLLuLG', 'true', 1, 2008, 5479, 'LPT1', 3171, 195, 997, 'E', 'MylmbMMyMlyMylby000yMl'),
(2, 'DFDFDD', 'Iapp', 3, 2007, 6484, 'GSGGyOgyySOOGygSOOgyOOyG', 275, NULL, 820, 'u4byb', ''),
(3, 'wFFwpEFEEwEwww', '', 2, 2006, 9836, NULL, 150, 343, 2404, 'WW', 'hDz');
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
(1, 1, '2cgugfgSS2Hc2', 'NaN', 'nil', 'M', '-KB K', 'False'),
(2, 1, 'jnIyjrnj4', 'false', '9', 'RzzKKrB', 'wT8', '');
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
(1, 2, 'vvv', 'LPT1', 2, 171, 'PGJHEJ', 352, 7095, NULL, 'cckv8crvFvJk8', NULL),
(2, 2, 'null', '-Infinity', 2, 288, 'm66akam36k33m', 291, 406, 977, 'QtQttjWyQjWyjWyH', 'Hg');
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
(1, 1, 2, 2, '(voice)', NULL, 2),
(2, 1, 2, 2, '(voice)', 0, 1),
(3, 1, 3, 2, '(voice)', 160, 2);
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
(1, 3, 3, 1),
(2, 2, 2, 2),
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
(1, 1, 2, 'USA', '3zjRQzYz'),
(2, 2, 3, 'Bulgaria', '0'),
(3, 3, 2, 'USA', 'else');
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
(1, 1, 1, '4.0', 'Jkrs'),
(2, 1, 3, '4.0', '-u'),
(3, 1, 2, '8.0', 'COM1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3),
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
(1, 2, 1, 1),
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
(1, 1, 2, 'False', '00'),
(2, 1, 1, 'wdO-Xdd', 'o9o99o'),
(3, 1, 3, 'vpn', 'nil');
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
(1, '', '[ru]', 1165, 'O''n', 'aq_q88Ya_N', NULL),
(2, 'hc', '[ru]', 631, 'NIL', '-Infinity', 'nil');
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
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'True'),
(2, 'hero-sequel', 'NtWNWW'),
(3, 'character-name-in-title', 'LPT1');
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
(1, 'Downey Jr., Robert', NULL, 276, '7wFw', 'FuguREFg__8', 'CjCLjCCLCejkeCzz''eb''ejCjCCC', NULL, 'b');
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
(1, 'Other Character', '0', NULL, 'uGsDssGGD1', 'MgMggM', 'LPT1'),
(2, 'Other Character', 's', 9575, 'pmpY_mppmmY__m_mm_mT_pTYTmTYYmm_', '_ww_wwww', '');
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
(1, 'MMbbtTLLuLG', 'true', 1, 2008, 5479, 'LPT1', 3171, 195, 997, 'E', 'MylmbMMyMlyMylby000yMl'),
(2, 'DFDFDD', 'Iapp', 3, 2007, 6484, 'GSGGyOgyySOOGygSOOgyOOyG', 275, NULL, 820, '', NULL),
(3, '1', '1', 2, NULL, 2006, '1', 150, 343, 2404, 'WW', 'hDz');
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
(1, 1, '2cgugfgSS2Hc2', 'NaN', 'nil', 'M', '-KB K', 'False'),
(2, 1, 'jnIyjrnj4', 'false', '9', 'RzzKKrB', 'wT8', '');
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
(1, 2, 'vvv', 'LPT1', 2, 171, 'PGJHEJ', 352, 7095, NULL, 'cckv8crvFvJk8', NULL),
(2, 2, 'null', '-Infinity', 2, 288, 'm66akam36k33m', 291, 406, 977, 'QtQttjWyQjWyjWyH', 'Hg');
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
(1, 1, 2, 2, '(voice)', NULL, 2),
(2, 1, 2, 2, '(voice)', 0, 1),
(3, 1, 3, 2, '(voice)', 160, 2);
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
(1, 3, 3, 1),
(2, 2, 2, 2),
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
(1, 1, 2, 'USA', '3zjRQzYz'),
(2, 2, 3, 'Bulgaria', '0'),
(3, 3, 2, 'USA', 'else');
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
(1, 1, 1, '4.0', 'Jkrs'),
(2, 1, 3, '4.0', '-u'),
(3, 1, 2, '8.0', 'COM1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3),
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
(1, 2, 1, 1),
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
(1, 1, 2, 'False', '00'),
(2, 1, 1, 'wdO-Xdd', 'o9o99o'),
(3, 1, 3, 'vpn', 'nil');
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
(1, '', '[ru]', 1165, 'O''n', 'aq_q88Ya_N', NULL),
(2, 'hc', '[ru]', 631, 'NIL', '-Infinity', 'nil');
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
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'True'),
(2, 'hero-sequel', 'NtWNWW'),
(3, 'character-name-in-title', 'LPT1');
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
(1, 'Downey Jr., Robert', NULL, 276, '7wFw', 'FuguREFg__8', 'CjCLjCCLCejkeCzz''eb''ejCjCCC', NULL, 'b');
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
(1, 'Other Character', '0', NULL, 'uGsDssGGD1', 'MgMggM', 'LPT1'),
(2, 'Other Character', 's', 9575, 'pmpY_mppmmY__m_mm_mT_pTYTmTYYmm_', '_ww_wwww', '');
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
(1, 'MMbbtTLLuLG', 'true', 1, 2008, 1, 'LPT1', 3171, 195, 997, 'E', 'MylmbMMyMlyMylby000yMl'),
(2, 'DFDFDD', 'Iapp', 3, 2007, 6484, 'GSGGyOgyySOOGygSOOgyOOyG', 275, NULL, 820, 'u4byb', ''),
(3, 'wFFwpEFEEwEwww', '', 2, 2006, 9836, NULL, 150, 343, 2404, 'WW', 'hDz');
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
(1, 1, '2cgugfgSS2Hc2', 'NaN', 'nil', 'M', '-KB K', 'False'),
(2, 1, 'jnIyjrnj4', 'false', '9', 'RzzKKrB', 'wT8', '');
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
(1, 2, 'vvv', 'LPT1', 2, 171, 'PGJHEJ', 352, 7095, NULL, 'cckv8crvFvJk8', NULL),
(2, 2, 'null', '-Infinity', 2, 288, 'm66akam36k33m', 291, 406, 977, 'QtQttjWyQjWyjWyH', 'Hg');
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
(1, 1, 2, 2, '(voice)', NULL, 2),
(2, 1, 2, 2, '(voice)', 0, 1),
(3, 1, 3, 2, '(voice)', 160, 2);
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
(1, 3, 3, 1),
(2, 2, 2, 2),
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
(1, 1, 2, 'USA', '3zjRQzYz'),
(2, 2, 3, 'Bulgaria', '0'),
(3, 3, 2, 'USA', 'else');
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
(1, 1, 1, '4.0', 'Jkrs'),
(2, 1, 3, '4.0', '-u'),
(3, 1, 2, '8.0', 'COM1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3),
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
(1, 2, 1, 1),
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
(1, 1, 2, 'False', '00'),
(2, 1, 1, 'wdO-Xdd', 'o9o99o'),
(3, 1, 3, 'vpn', 'nil');
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
(1, '', '[ru]', 1165, 'O''n', 'aq_q88Ya_N', NULL),
(2, 'hc', '[ru]', 631, 'NIL', '-Infinity', 'nil');
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
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'True'),
(2, 'hero-sequel', 'NtWNWW'),
(3, 'character-name-in-title', 'LPT1');
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
(1, 'Downey Jr., Robert', NULL, 276, '7wFw', 'FuguREFg__8', 'CjCLjCCLCejkeCzz''eb''ejCjCCC', NULL, 'b');
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
(1, 'Other Character', '0', NULL, 'uGsDssGGD1', 'MgMggM', 'LPT1'),
(2, 'Other Character', 's', 9575, 'pmpY_mppmmY__m_mm_mT_pTYTmTYYmm_', '_ww_wwww', '');
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
(1, 'MMbbtTLLuLG', 'true', 1, 2008, 5479, 'LPT1', 3171, 195, 997, 'E', 'MylmbMMyMlyMylby000yMl'),
(2, 'DFDFDD', 'Iapp', 3, 2007, 6484, 'GSGGyOgyySOOGygSOOgyOOyG', 275, NULL, 820, 'u4byb', ''),
(3, 'wFFwpEFEEwEwww', '', 2, 2006, 9836, NULL, 150, 343, 2404, 'WW', 'hDz');
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
(1, 1, '2cgugfgSS2Hc2', 'NaN', 'nil', 'M', '-KB K', 'False'),
(2, 1, 'jnIyjrnj4', 'false', '9', 'RzzKKrB', 'wT8', '');
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
(1, 2, 'vvv', 'LPT1', 2, 171, 'PGJHEJ', 352, 7095, NULL, 'cckv8crvFvJk8', NULL),
(2, 2, 'null', '-Infinity', 2, 288, 'm66akam36k33m', 291, 406, 977, 'QtQttjWyQjWyjWyH', 'Hg');
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
(1, 1, 2, 2, '(voice)', NULL, 2),
(2, 1, 2, 2, '(voice)', 0, 1),
(3, 1, 3, 2, '(voice)', 160, 2);
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
(1, 3, 3, 1),
(2, 2, 2, 2),
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
(1, 1, 2, 'USA', '3zjRQzYz'),
(2, 2, 3, 'Bulgaria', '0'),
(3, 3, 2, 'USA', 'NtWNWW');
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
(1, 1, 1, '4.0', 'Jkrs'),
(2, 1, 3, '4.0', '-u'),
(3, 1, 2, '8.0', 'COM1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3),
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
(1, 2, 1, 1),
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
(1, 1, 2, 'False', '00'),
(2, 1, 1, 'wdO-Xdd', 'o9o99o'),
(3, 1, 3, 'vpn', 'nil');
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
(1, '', '[ru]', 1165, 'O''n', 'aq_q88Ya_N', NULL),
(2, 'hc', '[ru]', 631, 'NIL', '-Infinity', 'nil');
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
(3, 'rating');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'marvel-cinematic-universe', 'True'),
(2, 'hero-sequel', 'NtWNWW'),
(3, 'character-name-in-title', 'LPT1');
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
(1, 'Downey Jr., Robert', NULL, 276, '7wFw', 'FuguREFg__8', 'CjCLjCCLCejkeCzz''eb''ejCjCCC', NULL, 'b');
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
(1, 'Other Character', '0', NULL, 'uGsDssGGD1', 'MgMggM', 'LPT1'),
(2, 'Other Character', 'RzzKKrB', 9575, 'pmpY_mppmmY__m_mm_mT_pTYTmTYYmm_', '_ww_wwww', '');
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
(1, 'MMbbtTLLuLG', 'true', 1, 2008, 5479, 'LPT1', 3171, 195, 997, 'E', 'MylmbMMyMlyMylby000yMl'),
(2, 'DFDFDD', 'Iapp', 3, 2007, 6484, 'GSGGyOgyySOOGygSOOgyOOyG', 275, NULL, 820, 'u4byb', ''),
(3, 'wFFwpEFEEwEwww', '', 2, 2006, 9836, NULL, 150, 343, 2404, 'WW', 'hDz');
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
(1, 1, '2cgugfgSS2Hc2', 'NaN', 'nil', 'M', '-KB K', 'False'),
(2, 1, 'jnIyjrnj4', 'false', '9', 'RzzKKrB', 'wT8', '');
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
(1, 2, 'vvv', 'LPT1', 2, 171, 'PGJHEJ', 352, 7095, NULL, 'cckv8crvFvJk8', NULL),
(2, 2, 'null', '-Infinity', 2, 288, 'm66akam36k33m', 291, 406, 977, 'QtQttjWyQjWyjWyH', 'Hg');
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
(1, 1, 2, 2, '(voice)', NULL, 2),
(2, 1, 2, 2, '(voice)', 0, 1),
(3, 1, 3, 2, '(voice)', 160, 2);
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
(1, 3, 3, 1),
(2, 2, 2, 2),
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
(1, 1, 2, 'USA', '3zjRQzYz'),
(2, 2, 3, 'Bulgaria', '0'),
(3, 3, 2, 'USA', 'else');
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
(1, 1, 1, '4.0', 'Jkrs'),
(2, 1, 3, '4.0', '-u'),
(3, 1, 2, '8.0', 'COM1');
CREATE TABLE movie_keyword (
  id BIGINT NOT NULL,
  movie_id BIGINT NOT NULL,
  keyword_id BIGINT NOT NULL,
  PRIMARY KEY (id),
  FOREIGN KEY (movie_id) REFERENCES title(id),
  FOREIGN KEY (keyword_id) REFERENCES keyword(id)
);
INSERT INTO movie_keyword VALUES
(1, 1, 3),
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
(1, 2, 1, 1),
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
(1, 1, 2, 'False', '00'),
(2, 1, 1, 'wdO-Xdd', 'o9o99o'),
(3, 1, 3, 'vpn', 'nil');
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
(1, 'FgFF8FTjgFoT8', NULL, NULL, 'False', '7oBmcmcBBoo%WX%7XBmco%m', 'false'),
(2, 'nil', '[ru]', 3, 'if', '', '__ '),
(3, 'IV-e', '[ru]', 4, 'ZFtZT', 'Y', 'false');
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
(1, 'countries');
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
(1, 'episode');
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
(1, 'Other Person', 'false', 64, 'true', 'XDDDD', NULL, 'Nk-N8NF', 'faaaafIffIfIfI'),
(2, 'Downey Jr., Robert', 'NUL', 512, '', 'b7bmbzSM', 'NULL', '', 'if'),
(3, 'Downey Jr., Robert', 'Scunthorpe', 15, 'oD', '', 'RxFleOMl', 'xxx444xxx', '0');
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
(1, 'Other Character', '', 128, '', 'True', 'bbbbbb');
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
(1, 'COM1', NULL, 1, 2009, 512, 'INF', 8191, 16, 127, 'false', NULL),
(2, '''z', NULL, 1, 2012, 31, '0', 178, 512, 8191, 'ftt', '');
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
(1, 3, 'true', 'WaaaWqe', NULL, 'false', 'GG', '__dict__');
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
(1, 1, 'yyNY', 'DMiQDMiiiMiDDMfifDDi', 1, 953, NULL, NULL, 3, 535, 'LLTL%%', '');
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
(1, 3, 1, 1, '(uncredited)', 1167, 1),
(2, 3, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(1, 2, 3, 2),
(2, NULL, 1, 1),
(3, 1, 1, 3);
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
(1, 2, 3, 2, 'NULL'),
(2, 1, 2, 2, 'None'),
(3, 1, 1, 2, 'R9');
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
(1, 2, 1, 'Bulgaria', 'U3rU'),
(2, 2, 1, 'Bulgaria', 'YZBB-Zg3j83jj'),
(3, 1, 1, 'USA', 'aauu');
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
(1, 2, 1, '8.0', 'LPT1'),
(2, 2, 1, '4.0', 'undefined'),
(3, 2, 1, '4.0', 'u2K');
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
(1, 2, 2, 1),
(2, 2, 1, 3),
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
(1, 1, 1, '', '0'),
(2, 1, 1, '', 'FALSE');
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
(1, 'FgFF8FTjgFoT8', NULL, NULL, 'False', '7oBmcmcBBoo%WX%7XBmco%m', 'false'),
(2, 'nil', '[ru]', 3, 'if', '', '__ '),
(3, 'IV-e', '[ru]', 4, 'ZFtZT', 'Y', 'false');
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
(1, 'countries');
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
(1, 'episode');
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
(1, 'Other Person', 'false', 64, 'true', 'XDDDD', NULL, 'Nk-N8NF', 'faaaafIffIfIfI'),
(2, 'Downey Jr., Robert', 'NUL', 512, '', 'b7bmbzSM', 'NULL', '', 'if'),
(3, 'Downey Jr., Robert', 'Scunthorpe', 15, 'oD', '', 'RxFleOMl', 'xxx444xxx', '0');
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
(1, 'Other Character', '', 128, '', 'True', 'bbbbbb');
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
(1, 'COM1', NULL, 1, 2009, 512, 'INF', 8191, 16, 127, 'false', NULL),
(2, '''z', NULL, 1, 2012, 31, '0', 178, 512, 8191, 'ftt', '');
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
(1, 3, 'true', 'WaaaWqe', NULL, 'false', 'GG', '__dict__');
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
(1, 1, 'yyNY', 'DMiQDMiiiMiDDMfifDDi', 1, 953, NULL, NULL, 3, 535, 'LLTL%%', '');
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
(1, 3, 1, 1, '(uncredited)', 1167, 1),
(2, 3, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(1, 2, 3, 2),
(2, NULL, 1, 1),
(3, 1, 1, 3);
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
(1, 2, 3, 2, 'NULL'),
(2, 1, 2, 2, 'None'),
(3, 2, 1, 2, 'R9');
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
(1, 2, 1, 'Bulgaria', 'U3rU'),
(2, 2, 1, 'Bulgaria', 'YZBB-Zg3j83jj'),
(3, 1, 1, 'USA', 'aauu');
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
(1, 2, 1, '8.0', 'LPT1'),
(2, 2, 1, '4.0', 'undefined'),
(3, 2, 1, '4.0', 'u2K');
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
(1, 2, 2, 1),
(2, 2, 1, 3),
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
(1, 1, 1, '', '0'),
(2, 1, 1, '', 'FALSE');
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
(1, 'FgFF8FTjgFoT8', NULL, NULL, 'False', '7oBmcmcBBoo%WX%7XBmco%m', 'false'),
(2, 'nil', '[ru]', 3, 'if', '', '__ '),
(3, 'IV-e', '[ru]', 4, 'ZFtZT', 'Y', 'false');
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
(1, 'countries');
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
(1, 'Downey Jr., Robert', NULL, NULL, 'false', '1', NULL, NULL, 'XDDDD'),
(2, 'Other Person', 'Nk-N8NF', 1, NULL, NULL, 'NUL', '1', ''),
(3, 'Other Person', NULL, 1, NULL, '', 'if', NULL, 'Scunthorpe');
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
(1, 'Other Character', NULL, NULL, '', 'RxFleOMl', 'xxx444xxx');
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
(1, '1', NULL, 1, NULL, NULL, NULL, 1, 128, 1, 'True', 'bbbbbb'),
(2, 'COM1', NULL, 1, 2009, 512, 'INF', 8191, 16, 127, 'false', NULL);
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
(1, 2, '1', '1', NULL, NULL, '0', '1');
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
(1, 2, '', NULL, 1, NULL, NULL, 1, NULL, NULL, NULL, 'WaaaWqe');
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
(1, 2, 2, NULL, '(voice)', NULL, 2),
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
(1, NULL, 2, 1),
(2, 2, 2, 2),
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
(1, 2, 2, 2, ''),
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
(1, 1, 1, 'USA', '1'),
(2, 2, 1, 'USA', '1'),
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
(1, 2, 1, '4.0', NULL),
(2, 2, 1, '4.0', 'NULL'),
(3, 1, 1, '4.0', 'None');
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
(2, 1, 2, 1),
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
(1, 1, 1, '', NULL),
(2, 2, 1, '1', 'aauu');
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
(1, 'FgFF8FTjgFoT8', NULL, NULL, 'False', '7oBmcmcBBoo%WX%7XBmco%m', 'false'),
(2, 'nil', '[ru]', 3, 'if', '', '__ '),
(3, 'IV-e', '[ru]', 4, 'ZFtZT', 'Y', 'false');
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
(1, 'countries');
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
(1, 'episode');
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
(1, 'Other Person', 'false', 64, 'true', 'XDDDD', NULL, 'Nk-N8NF', 'faaaafIffIfIfI'),
(2, 'Downey Jr., Robert', 'Scunthorpe', 512, '', 'b7bmbzSM', 'NULL', '', 'if'),
(3, 'Downey Jr., Robert', 'Scunthorpe', 15, 'oD', '', 'RxFleOMl', 'xxx444xxx', '0');
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
(1, 'Other Character', '', 128, '', 'True', 'bbbbbb');
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
(1, 'COM1', NULL, 1, 2009, 512, 'INF', 8191, 16, 127, 'false', NULL),
(2, '''z', NULL, 1, 2012, 31, '0', 178, 512, 8191, 'ftt', '');
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
(1, 3, 'true', 'WaaaWqe', NULL, 'false', 'GG', '__dict__');
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
(1, 1, 'yyNY', 'DMiQDMiiiMiDDMfifDDi', 1, 953, NULL, NULL, 3, 535, 'LLTL%%', '');
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
(1, 3, 1, 1, '(uncredited)', 1167, 1),
(2, 3, 2, 1, '(voice) (uncredited)', NULL, 1);
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
(1, 2, 3, 2),
(2, NULL, 1, 1),
(3, 1, 1, 3);
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
(1, 2, 3, 2, 'NULL'),
(2, 1, 2, 2, 'None'),
(3, 1, 1, 2, 'R9');
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
(1, 2, 1, 'Bulgaria', 'U3rU'),
(2, 2, 1, 'Bulgaria', 'YZBB-Zg3j83jj'),
(3, 1, 1, 'USA', 'aauu');
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
(1, 2, 1, '8.0', 'LPT1'),
(2, 2, 1, '4.0', 'undefined'),
(3, 2, 1, '4.0', 'u2K');
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
(1, 2, 2, 1),
(2, 2, 1, 3),
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
(1, 1, 1, '', '0'),
(2, 1, 1, '', 'FALSE');
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
(1, 'None', '[ru]', 1018, '', NULL, '0'),
(2, 'TTt', '[us]', 1, NULL, 'uL5HLou', 'dddddd'),
(3, 'J', '[us]', 266, '0', 'SMS4SpGpW44', 'False');
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
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'kw8fxkffwx8x8');
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
(1, 'Downey Jr., Robert', '%%%%%%%%%%%%', 1, 'RRRRRRHHHHRRRHRRHRH', 'QwCNQCN', 'ytUX', 'c', 'NIL');
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
(1, 'Voice Character', 'V38B', 2664, 'COM1', 'False', ' Wn FnPnP'),
(2, 'Other Character', 'nil', 703, 'Xlly', 'l_wwG', 'yOyy'),
(3, 'Other Character', '%Bt_TBIIr%', 2047, '0', 'fJJp', 'A');
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
(1, 'Y88''RL', '55', 2, 2011, 420, NULL, 10000, 1024, 9999, '6him7hh', NULL),
(2, '999999999999999999999999999999', '2323''2', 2, 2010, 0, 'OAo', 60, 0, 1, 'then', ''),
(3, 'T', 'd5', 1, 2009, NULL, 'TRUE', 5914, 0, 10000, 'COM1', 'LPT1');
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
(1, 1, 'sss', '__proto__', 'kpkI1kIzzzvzI9J9zJ', 'None', 'INF', 'D');
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
(1, 3, '0', NULL, 2, 10000, '', 0, 10000, NULL, 'xxs', 'LzzDlDhh');
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
(1, 1, 1, 2, '(voice)', 10000, 1),
(2, 1, 2, 1, '(uncredited)', 9999, 3);
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
(1, 1, 3, 3);
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
(1, 3, 1, 2, 'NULL');
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
(1, 2, 2, 'USA', 'P0c'),
(2, 1, 2, 'Bulgaria', 'False');
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
(1, 1, 2, '8.0', 'jJjPGGejjeePuJR'),
(2, 1, 2, '4.0', 'OH'),
(3, 2, 2, '4.0', 'aa');
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
(1, 3, 1, 2),
(2, 3, 1, 3);
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
(1, 1, 1, 'LdUdL2UUd6dyy', 'Inf'),
(2, 1, 1, 'NaN', 'kFhk6hFF6xx6_x_Gk'),
(3, 1, 2, '22uLrzLu2zurrLLz', 'juu');
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
(1, 'None', '[ru]', 1018, '', NULL, '0'),
(2, 'TTt', '[us]', 1, NULL, 'uL5HLou', 'dddddd'),
(3, 'J', '[us]', 266, '0', 'SMS4SpGpW44', 'False');
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
(1, 'rating'),
(2, 'countries');
CREATE TABLE keyword (
  id BIGINT NOT NULL,
  keyword VARCHAR NOT NULL,
  phonetic_code VARCHAR,
  PRIMARY KEY (id)
);
INSERT INTO keyword VALUES
(1, 'character-name-in-title', 'kw8fxkffwx8x8');
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
(1, 'Other Person', '%%%%%%%%%%%%', 1, 'RRRRRRHHHHRRRHRRHRH', 'QwCNQCN', 'ytUX', 'c', 'NIL');
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
(1, 'Voice Character', 'V38B', 2664, 'COM1', 'False', ' Wn FnPnP'),
(2, 'Other Character', 'nil', 703, 'Xlly', 'l_wwG', 'yOyy'),
(3, 'Other Character', '%Bt_TBIIr%', 2047, '0', 'fJJp', 'A');
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
(1, 'Y88''RL', '55', 2, 2011, 420, NULL, 10000, 1024, 9999, '6him7hh', NULL),
(2, '999999999999999999999999999999', '2323''2', 2, 2010, 0, 'OAo', 60, 0, 1, 'then', ''),
(3, 'T', 'd5', 1, 2009, NULL, 'TRUE', 5914, 0, 10000, 'COM1', 'LPT1');
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
(1, 1, 'sss', '__proto__', 'kpkI1kIzzzvzI9J9zJ', 'None', 'INF', 'D');
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
(1, 3, '0', NULL, 2, 10000, '', 0, 10000, NULL, 'xxs', 'LzzDlDhh');
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
(1, 1, 1, 2, '(voice)', 10000, 1),
(2, 1, 2, 1, '(uncredited)', 9999, 3);
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
(1, 1, 3, 3);
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
(1, 3, 1, 2, 'NULL');
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
(1, 2, 2, 'USA', 'P0c'),
(2, 1, 2, 'Bulgaria', 'False');
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
(1, 1, 2, '8.0', 'jJjPGGejjeePuJR'),
(2, 1, 2, '4.0', 'OH'),
(3, 2, 2, '4.0', 'aa');
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
(1, 3, 1, 2),
(2, 3, 1, 3);
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
(1, 1, 1, 'LdUdL2UUd6dyy', 'Inf'),
(2, 1, 1, 'NaN', 'kFhk6hFF6xx6_x_Gk'),
(3, 1, 2, '22uLrzLu2zurrLLz', 'juu');
ROLLBACK;

