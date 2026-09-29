
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.OwnerUserId,
        p.Title,
        p.CreationDate,
        COUNT(c.Id) AS CommentCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS RankByComments
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
    GROUP BY 
        p.Id, p.OwnerUserId, p.Title, p.CreationDate
),
TopPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.OwnerUserId,
        (rp.UpVotes - rp.DownVotes) AS NetVotes,
        rp.CommentCount
    FROM 
        RankedPosts rp
    WHERE 
        rp.RankByComments = 1
)
SELECT 
    u.DisplayName,
    tp.Title,
    tp.NetVotes,
    tp.CommentCount,
    CASE 
        WHEN tp.NetVotes > 100 THEN 'Hot Post'
        WHEN tp.NetVotes BETWEEN 50 AND 100 THEN 'Trending Post'
        ELSE 'Regular Post'
    END AS PostCategory,
    COALESCE(b.Name, 'No Badge') AS UserBadge
FROM 
    TopPosts tp
JOIN 
    Users u ON tp.OwnerUserId = u.Id
LEFT JOIN 
    Badges b ON b.UserId = u.Id AND b.Class = 1
WHERE 
    u.Reputation > 1000
ORDER BY 
    tp.NetVotes DESC, tp.PostId, UserBadge
LIMIT 10;
