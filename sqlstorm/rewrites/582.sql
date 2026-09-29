
WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.ViewCount,
        p.CreationDate,
        p.PostTypeId,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id, a.Id) AS RankByViews,
        COALESCE(a.OwnerUserId, -1) AS AnswerOwnerId
    FROM 
        Posts p
    LEFT JOIN 
        Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
    WHERE 
        p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
),
TopPosts AS (
    SELECT 
        rp.Id,
        rp.Title,
        rp.ViewCount,
        rp.RankByViews,
        rp.AnswerOwnerId
    FROM 
        RankedPosts rp
    WHERE 
        rp.RankByViews <= 10
),
UserStatistics AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS TotalPosts,
        SUM(v.BountyAmount) AS TotalBountyAwarded,
        AVG(u.Reputation) AS AvgReputation
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) 
    GROUP BY 
        u.Id, u.DisplayName
)
SELECT 
    tp.Title,
    tp.ViewCount,
    us.DisplayName,
    us.TotalPosts,
    us.TotalBountyAwarded,
    CONCAT('@', us.DisplayName) AS TwitterHandle,
    CASE 
        WHEN us.TotalPosts > 20 THEN 'Active'
        WHEN us.TotalPosts BETWEEN 10 AND 20 THEN 'Moderately Active'
        ELSE 'Less Active'
    END AS ActivityStatus,
    COALESCE(us.AvgReputation, 0) AS ReputationScore
FROM 
    TopPosts tp
JOIN 
    UserStatistics us ON us.UserId = tp.AnswerOwnerId
ORDER BY 
    tp.ViewCount DESC, us.TotalBountyAwarded DESC, tp.Id, us.UserId
LIMIT 10
OFFSET 5;
