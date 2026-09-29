WITH RankedPosts AS (
    SELECT p.Id AS PostId, 
           p.Title, 
           p.CreationDate, 
           p.Score, 
           u.DisplayName AS OwnerName,
           ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS rn
    FROM Posts p
    LEFT JOIN Users u ON p.OwnerUserId = u.Id
    WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
AggregateVotes AS (
    SELECT PostId, 
           SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
           SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
           COUNT(VoteTypeId) AS TotalVotes
    FROM Votes
    GROUP BY PostId
),
TopPosts AS (
    SELECT rp.PostId, 
           rp.Title, 
           rp.CreationDate, 
           rp.Score, 
           rp.OwnerName, 
           av.UpVotes, 
           av.DownVotes, 
           av.TotalVotes
    FROM RankedPosts rp
    LEFT JOIN AggregateVotes av ON rp.PostId = av.PostId
    WHERE rp.rn = 1
)
SELECT tp.PostId,
       tp.Title,
       tp.OwnerName,
       COALESCE(tp.UpVotes, 0) - COALESCE(tp.DownVotes, 0) AS NetVotes,
       tp.CreationDate,
       CASE 
           WHEN tp.Score > 10 THEN 'Highly Rated'
           WHEN tp.Score BETWEEN 5 AND 10 THEN 'Moderately Rated'
           ELSE 'Low Rated'
       END AS RatingCategory
FROM TopPosts tp
WHERE (tp.Score > 0 OR (tp.OwnerName IS NOT NULL AND tp.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'))
ORDER BY NetVotes DESC, tp.CreationDate DESC, tp.PostId
LIMIT 100;