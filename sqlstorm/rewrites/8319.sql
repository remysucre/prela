WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.OwnerUserId,
        p.Score,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS RankByScore
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
        AND p.PostTypeId = 1 
),
UserVotes AS (
    SELECT 
        v.UserId,
        COUNT(v.Id) AS VoteCount,
        SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
    FROM 
        Votes v
    JOIN 
        VoteTypes vt ON v.VoteTypeId = vt.Id
    GROUP BY 
        v.UserId
),
UserInformation AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COALESCE(b.BatchCount, 0) AS BatchCount,
        COALESCE(uv.VoteCount, 0) AS UserVoteCount,
        COALESCE(uv.UpVotes, 0) AS UserUpVotes,
        COALESCE(uv.DownVotes, 0) AS UserDownVotes
    FROM 
        Users u
    LEFT JOIN (
        SELECT 
            UserId,
            COUNT(Id) AS BatchCount
        FROM 
            Badges
        GROUP BY 
            UserId
    ) b ON u.Id = b.UserId
    LEFT JOIN UserVotes uv ON u.Id = uv.UserId
)
SELECT 
    p.PostId,
    p.Title,
    p.CreationDate,
    u.DisplayName,
    u.UserVoteCount,
    u.UserUpVotes,
    u.UserDownVotes,
    RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS GlobalRank
FROM 
    RankedPosts p
JOIN 
    UserInformation u ON p.OwnerUserId = u.UserId
WHERE 
    p.RankByScore <= 5
ORDER BY 
    GlobalRank, p.Score DESC;