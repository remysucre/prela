WITH RankedPosts AS (
 SELECT
 p.Id AS PostId,
 p.Title,
 p.CreationDate,
 p.Score,
 p.ViewCount,
 COUNT(c.Id) AS CommentCount,
 SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
 SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
 ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
 FROM
 Posts p
 LEFT JOIN
 Comments c ON p.Id = c.PostId
 LEFT JOIN
 Votes v ON p.Id = v.PostId
 WHERE
 p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
 GROUP BY
 p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId
),
TopPosts AS (
 SELECT
 rp.PostId,
 rp.Title,
 rp.CreationDate,
 rp.Score,
 rp.ViewCount,
 rp.CommentCount,
 rp.UpVotes,
 rp.DownVotes
 FROM
 RankedPosts rp
 WHERE
 rp.PostRank <= 10
)
SELECT
 tp.PostId,
 tp.Title,
 tp.CreationDate,
 tp.Score,
 tp.ViewCount,
 tp.CommentCount,
 tp.UpVotes,
 tp.DownVotes,
 COALESCE((SELECT STRING_AGG(t.TagName, ', ' ORDER BY t.TagName)
 FROM Posts p
 JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%'
 WHERE p.Id = tp.PostId), 'No Tags') AS Tags
FROM
 TopPosts tp
ORDER BY
 tp.Score DESC, tp.ViewCount DESC;
